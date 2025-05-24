use futures::StreamExt;
use rabbitmq_stream_client::{
    Environment,
    error::StreamCreateError,
    types::{ByteCapacity, Message, OffsetSpecification, ResponseCode},
};
use service::{FormData, ResponseData, sandbox_service};
use std::sync::Arc;
use tokio::sync::Mutex;

mod service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build Consumer
    let environment = Environment::builder().build().await?;
    let receive_stream = "Server2Runner";
    let create_response = environment
        .stream_creator()
        .max_length(ByteCapacity::GB(1))
        .create(receive_stream)
        .await;
    if let Err(e) = create_response {
        if let StreamCreateError::Create { stream, status } = e {
            match status {
                ResponseCode::StreamAlreadyExists => {}
                err => {
                    println!("Error creating stream: {:?} {:?}", stream, err);
                }
            }
        }
    }
    let mut consumer = environment
        .consumer()
        .offset(OffsetSpecification::Next)
        .build(receive_stream)
        .await
        .unwrap();
    // Build Producer
    let send_stream = "Runner2Server";
    let create_response = environment
        .stream_creator()
        .max_length(ByteCapacity::GB(1))
        .create(send_stream)
        .await;
    if let Err(e) = create_response {
        if let StreamCreateError::Create { stream, status } = e {
            match status {
                ResponseCode::StreamAlreadyExists => {}
                err => {
                    println!("Error creating stream: {:?} {:?}", stream, err);
                }
            }
        }
    }
    let producer = Arc::new(Mutex::new(environment.producer().build(send_stream).await?));
    while let Some(delivery) = consumer.next().await {
        let d = delivery.unwrap();
        let message = d
            .message()
            .data()
            .map(|data| String::from_utf8(data.to_vec()))
            .unwrap()
            .unwrap();
        print!("{}", message);
        let form_data: FormData = serde_yaml::from_str(&message).unwrap();
        let commands = form_data.commands.clone();
        let image = form_data.image;
        let result = match sandbox_service(commands, image) {
            Ok(res) => res,
            Err(_) => {
                continue;
            }
        };
        let result = ResponseData {
            sandbox_results: result,
            submit_id: form_data.submit_id,
        };
        let producer = producer.clone();
        let message = Message::builder()
            .body(serde_yaml::to_string(&result).unwrap_or_default())
            .build();
        tokio::spawn(async move {
            producer
                .lock()
                .await
                .send_with_confirm(message)
                .await
                .unwrap();
        });
    }
    Ok(())
}

#[cfg(test)]
mod main_test {
    use rabbitmq_stream_client::{
        Environment,
        error::StreamCreateError,
        types::{ByteCapacity, Message, ResponseCode},
    };

    use crate::service::{CMD, Config, FormData};

    #[tokio::test]
    async fn gcc_version() -> Result<(), Box<dyn std::error::Error>> {
        let environment = Environment::builder().build().await?;
        let stream = "Server2Runner";
        let create_response = environment
            .stream_creator()
            .max_length(ByteCapacity::GB(1))
            .create(stream)
            .await;

        if let Err(e) = create_response {
            if let StreamCreateError::Create { stream, status } = e {
                match status {
                    // we can ignore this error because the stream already exists
                    ResponseCode::StreamAlreadyExists => {}
                    err => {
                        println!("Error creating stream: {:?} {:?}", stream, err);
                    }
                }
            }
        }

        let producer = environment.producer().build(stream).await?;

        let commands = vec![CMD {
            command: "gcc".to_string(),
            args: vec!["--version".to_string()],
            input: "".to_string(),
            config: Config {
                time_limit: 1,
                time_reserved: 1,
                memory_limit: 256000,
                memory_reserved: 4096000,
                large_stack: false,
                output_limit: 0,
                process_limit: 0,
            },
        }];
        let form_data = FormData {
            commands,
            image: "gcc:14.2",
            submit_id: "......".to_string(),
        };

        producer
            .send_with_confirm(
                Message::builder()
                    .body(serde_yaml::to_string(&form_data).unwrap())
                    .build(),
            )
            .await?;
        println!("Sent message to stream: {}", stream);
        producer.close().await?;
        Ok(())
    }

    #[tokio::test]
    async fn cpp_a_add_b() -> Result<(), Box<dyn std::error::Error>> {
        let environment = Environment::builder().build().await?;
        let stream = "Server2Runner";
        let create_response = environment
            .stream_creator()
            .max_length(ByteCapacity::GB(1))
            .create(stream)
            .await;

        if let Err(e) = create_response {
            if let StreamCreateError::Create { stream, status } = e {
                match status {
                    // we can ignore this error because the stream already exists
                    ResponseCode::StreamAlreadyExists => {}
                    err => {
                        println!("Error creating stream: {:?} {:?}", stream, err);
                    }
                }
            }
        }

        let producer = environment.producer().build(stream).await?;

        let commands = vec![
            CMD {
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    r#"echo '#include <iostream>
using namespace std;
int main() {
    int a, b;
    cin >> a >> b;
    cout << a << " + " << b << " = " << a + b << endl;
}' > main.cpp"#
                        .to_string(),
                ],
                input: "".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 4096000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
            CMD {
                command: "g++".to_string(),
                args: vec!["main.cpp".to_string(), "-o".to_string(), "main".to_string()],
                input: "".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 4096000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
            CMD {
                command: "./main".to_string(),
                args: vec![],
                input: "1 2".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 4096000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
        ];
        let form_data = FormData {
            commands,
            image: "gcc:14.2",
            submit_id: "......".to_string(),
        };

        print!("{}", serde_yaml::to_string(&form_data).unwrap());

        producer
            .send_with_confirm(
                Message::builder()
                    .body(serde_yaml::to_string(&form_data).unwrap())
                    .build(),
            )
            .await?;
        println!("Sent message to stream: {}", stream);
        producer.close().await?;
        Ok(())
    }
}

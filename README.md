- Please Ensure Docker Daemon is Running

  请确保 Docker 守护进程正在运行

- Please Ensure RabbitMQ is running and Plugins are activated

  请确保 RabbitMQ 正在运行，并且插件已激活

  ```shell
  docker run -it --rm --name rabbitmq -p 5552:5552 -p 15672:15672 -p 5672:5672  \
    -e RABBITMQ_SERVER_ADDITIONAL_ERL_ARGS='-rabbitmq_stream advertised_host localhost' \
    rabbitmq:4-management
  docker exec rabbitmq rabbitmq-plugins enable rabbitmq_stream rabbitmq_stream_management
  ```

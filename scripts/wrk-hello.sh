# -> examples/hello-async-std
# -> examples/hello-tokio
wrk -c 256 -t 16 --latency http://localhost:8080/

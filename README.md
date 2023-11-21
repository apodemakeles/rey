## rey
The Rust version of [hey](https://github.com/rakyll/hey), a tool for sending load to web application, like ab.

The purpose of this project is purely for learning and understanding the Rust language in a more practical way.
We use [reqwest](https://github.com/seanmonstar/reqwest) as the HTTP client. Due to the nature of reqwest, certain low-level HTTP statistics such as detailed timing data are not currently supported.

## Usage
```
Usage: rey [OPTIONS] <URL>

Arguments:
  <URL>  

Options:
  -n <REQUESTS>               Name of the person to greet [default: 200]
  -c <WORKERS>                Number of workers to run concurrently. Total number of requests cannot be smaller than the concurrency level [default: 50]
  -q <RATE LIMIT>             Rate limit, in queries per second (QPS) per worker
  -z <Duration>               Duration of application to send requests. When duration is reached, application stops and exits. If duration is specified, n is ignored. Examples: -z 10s -z 3m
  -m <METHOD>                 HTTP method, one of GET, POST, PUT, DELETE, HEAD, OPTIONS [default: GET]
  -H <HEADERS>                Custom HTTP header. You can specify as many as needed by repeating the flag. For example, -H "Accept: text/html" -H "Content-Type: application/xml"
  -t <TIMEOUT>                Timeout for each request in seconds. Use 0 for infinite [default: 20]
  -A <ACCEPT HEADER>          HTTP Accept header
  -T <CONTENT-TYPE>           Content-type, defaults to "text/html" [default: text/html]
  -U <USER AGENT>             User-Agent, defaults to version "rey/0.1.0"
  -d <BODY>                   HTTP request body
  -D <FILE>                   HTTP request body from file. For example, /home/user/file.txt or ./file.txt
  -a <USERNAME:PASSWORD>      Basic authentication, username:password
  -x <PROXY>                  HTTP Proxy address as host:port
      --host <HOST>           
      --disable-redirects     
  -h, --help                  Print help
  -V, --version               Print version
```
## Output
```
Summary:
  Total:  3.1956 secs
  Slowest:  0.4934 secs
  Fastest:  0.0084 secs
  Average:  0.2426 secs
  Requests/sec:  31.2928
  
  Total data:   500 B bytes
  Size/request: 5 B bytes

Response time histogram:
0.008 [1]       |■■■
0.057 [15]      |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
0.105 [10]      |■■■■■■■■■■■■■■■■■■■■■■■■■
0.154 [6]       |■■■■■■■■■■■■■■■
0.202 [10]      |■■■■■■■■■■■■■■■■■■■■■■■■■
0.251 [8]       |■■■■■■■■■■■■■■■■■■■■
0.299 [10]      |■■■■■■■■■■■■■■■■■■■■■■■■■
0.348 [11]      |■■■■■■■■■■■■■■■■■■■■■■■■■■■■
0.396 [5]       |■■■■■■■■■■■■■
0.445 [16]      |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
0.493 [8]       |■■■■■■■■■■■■■■■■■■■■

Latency distribution: 
  10% in 0.0344 secs 
  25% in 0.0984 secs 
  50% in 0.2525 secs 
  75% in 0.3884 secs 
  90% in 0.4407 secs 
  95% in 0.4698 secs 
  99% in 0.4934 secs 

Status code distribution: 
  [200] 96 responses
  [400] 1 responses
  [409] 1 responses
  [500] 1 responses
  [502] 1 responses
```
## Roadmap
The purpose of this project is primarily for learning, and I won't be investing more energy into it at present. However, there is a possibility that the following features may be developed in the future:
+ Improve result formatting.
+ Support more detailed statistical data, may include DNS resolution times, connection times, response times, etc.
+ Refactor using a better concurrency pattern.
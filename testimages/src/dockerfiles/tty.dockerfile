FROM alpine

EXPOSE 8080
CMD [ "/bin/sh", "-c", "[[ -t 1 ]] && echo -en 'HTTP/1.0 200 OK\r\n\r\n' | nc -l -p 8080" ]

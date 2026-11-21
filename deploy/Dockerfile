FROM ubuntu:24.04
RUN install -d -o 10001 -g 10001 -m 0700 /var/lib/agentx-cloud
COPY --chmod=0755 agentx /usr/local/bin/agentx
COPY LICENSE /usr/share/doc/agentx/LICENSE
USER 10001:10001
ENTRYPOINT ["/usr/local/bin/agentx"]
CMD ["daemon","cloud","--root","/var/lib/agentx-cloud","--listen","127.0.0.1:18755","--public-url","http://127.0.0.1:18755"]

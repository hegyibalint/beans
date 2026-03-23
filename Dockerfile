FROM opensuse/tumbleweed:latest

RUN zypper -n refresh && zypper -n install \
    git jq ripgrep fd tmux \
    rust cargo \
    nodejs-default npm-default \
    glibc-locale-base \
    && zypper clean -a

ENV LANG=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8

RUN useradd -u 1000 -m dev \
    && mkdir -p /run/tmux/1000 && chown dev:dev /run/tmux/1000 && chmod 700 /run/tmux/1000
RUN npm install -g @anthropic-ai/claude-code

USER dev
RUN mkdir -p /home/dev/.claude
RUN echo '{"hasCompletedOnboarding":true,"theme":"dark"}' > /home/dev/.claude.json

COPY --chown=dev:users entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["entrypoint.sh"]
CMD ["bash"]

FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

ARG USERNAME=vscode

# Used to persist bash history as per https://code.visualstudio.com/remote/advancedcontainers/persist-bash-history
RUN SNIPPET="export PROMPT_COMMAND='history -a' && export HISTFILE=/commandhistory/.bash_history" \
  && mkdir /commandhistory \
  && touch /commandhistory/.bash_history \
  && chown -R $USERNAME /commandhistory \
  && echo "$SNIPPET" >> "/home/$USERNAME/.bashrc"

# Used to store Claude configuration files
RUN mkdir /home/$USERNAME/.claude && chown -R $USERNAME /home/$USERNAME/.claude
# Used to store Codex configuration files
RUN mkdir /home/$USERNAME/.codex && chown -R $USERNAME /home/$USERNAME/.codex

# Alias to run Claude with dangerously-skip-permissions flag
RUN echo "alias claude='claude --dangerously-skip-permissions'" >> /etc/bash.bashrc
# Alias to run Codex with dangerously-bypass-approvals-and-sandbox flag
RUN echo "alias codex='codex --dangerously-bypass-approvals-and-sandbox'" >> /etc/bash.bashrc

# Install Yices2 SMT solver dependencies
RUN apt-get update && apt-get install -y \
  gperf \
  libgmp-dev \
  wget \
  build-essential \
  autoconf \
  git \
  && rm -rf /var/lib/apt/lists/*

# Build and install Kissat SAT solver (with -fPIC for Yices2 compatibility)
RUN cd /tmp \
  && git clone https://github.com/BrunoDutertre/kissat \
  && cd kissat \
  && ./configure -fPIC \
  && make \
  && install build/libkissat.a /usr/local/lib \
  && install -m644 src/kissat.h /usr/local/include \
  && cd /tmp \
  && rm -rf kissat

# Build and install Yices2 from source with Kissat support
RUN cd /tmp \
  && git clone https://github.com/SRI-CSL/yices2 \
  && cd yices2 \
  && autoconf \
  && ./configure CPPFLAGS=-DHAVE_KISSAT LIBS='-lkissat -lm' \
  && make -j \
  && make install \
  && cd /tmp \
  && rm -rf yices2

# Set library path for Yices2
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

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

# Alias to run Claude with dangerously-skip-permissions flag
RUN echo "alias claude='claude --dangerously-skip-permissions'" >> /etc/bash.bashrc

# Install Yices2 SMT solver dependencies
RUN apt-get update && apt-get install -y \
  gperf \
  libgmp-dev \
  wget \
  build-essential \
  && rm -rf /var/lib/apt/lists/*

# Download and install Yices2
RUN cd /tmp \
  && wget https://yices.csl.sri.com/releases/2.6.4/yices-2.6.4-x86_64-pc-linux-gnu.tar.gz \
  && tar -xzf yices-2.6.4-x86_64-pc-linux-gnu.tar.gz \
  && cd yices-2.6.4 \
  && ./install-yices /usr/local \
  && rm -rf /tmp/yices-2.6.4*

# Set library path for Yices2
ENV LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH

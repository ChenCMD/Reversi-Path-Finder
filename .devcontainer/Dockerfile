FROM mcr.microsoft.com/devcontainers/java:1-21-bullseye

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

# Download Kotlin for VS Code
ARG KOTLIN_FOR_VSCODE_VERSION
RUN wget https://download-cdn.jetbrains.com/kotlin-lsp/$KOTLIN_FOR_VSCODE_VERSION/kotlin-$KOTLIN_FOR_VSCODE_VERSION.vsix -O /tmp/kotlin-for-vscode.vsix

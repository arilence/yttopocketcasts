# yttopocketcasts

Send Youtube videos as audio podcasts to your personal Pocket Casts files section.

## Quick Start

Prerequisites: [Docker](https://www.docker.com/) and a [way to run devcontainer](https://containers.dev/supporting).

1. Create a Telegram bot to receive an API token

2. Duplicate `.env.example` to `.env` or set corresponding environment variables.

3. Start the bot

    - Using VS Code:

        - Open the project folder in VS Code and use command palette: Reopen in Container
        - Wait for the container to finish building.
        - Open a new terminal _(Ctrl + Shift + \`)_ and execute `cargo run`.

    - Using devcontainers CLI:

        ```
        $ devcontainers up
        $ devcontainers exec cargo run
        ```

4. Use Telegram to send the bot command `/start`

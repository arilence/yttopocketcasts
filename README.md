# yttopocketcasts

Send Youtube videos as audio podcasts to your personal Pocket Casts files section.

## Quick Start

Prerequisites: Docker and Make must be installed

1. Create a Telegram bot to receive an API token

2. Set secrets through environment variables or by creating an `.env` file

    Example:

    ```
    [ ! -f .env ] && cat <<EOF > .env
    TELOXIDE_TOKEN=123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11  # Bot API token
    TRUSTED_USER_IDS=0000000000,0000000000  # Users who can use the bot
    ADMIN_USER_IDS=0000000000,0000000000    # Users who can run admin commands
    EOF
    ```

    Both `TRUSTED_USER_IDS` and `ADMIN_USER_IDS` are optional. Start the bot and send the command `/id` to retrieve your user id.

3. Start the bot

    ```
    make run
    ```

4. Send the command `/start` to the bot

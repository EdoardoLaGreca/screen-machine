# screen-machine
A bot/program that screenshots a MS Teams slide when it changes, saving the previous one and publishing it to a discord channel.

## Roles
`bot` = the actual Discord bot (client). The bot simply gets the images from the server and sends them in a discord channel.  
`backend` = the actual program behind the scenes (server). The backend screenshots the current window (make it be MS Teams just before starting it) and listens for connections. After the client connects (should be immediate), every time the backend gets a screenshot, it's sent to the client.  

The backend won't start taking screenshots until the client connects.

## How to start
First, start the Discord bot using this command (make sure you have a token, here it's hidden for obvious reasons).
```
python3 bot/bot.rs
```
Then, compile and run the backend.
```
cd backend/
cargo build --release --target-dir bin
./bin/backend
```
And finally, tell the bot that everything's ready (write this in a Discord channel where the bot is in).
```
vitto start [[<host>] <port>]
```
`host` = a custom (and optional) host.  
`port` = a custom (and optional) port.  
Note that you can't specify a port without also specifying a host. But you can specify a host without specifying a port.  
Default host and port can be found as global variables in `bot/bot.py`.

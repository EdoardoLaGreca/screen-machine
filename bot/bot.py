import discord # `pip install discord.py`
import websockets # `pip install websockets`
import time
import os
from PIL import Image

from dotenv import load_dotenv # `pip install python-dotenv`

load_dotenv()
TOKEN = os.getenv('DISCORD_TOKEN')

client = discord.Client()

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = "4444"
DEFAULT_ADDRESS = f"ws://{DEFAULT_HOST}:{DEFAULT_PORT}"
EXIT = False

# Send screenshot to client
async def send_screenshot(image, discord_channel):
    # Create a screenshot image and send it to the discord channel
    screenshot = Image.fromarray(image)
    await discord_channel.send(discord_channel, discord.File(screenshot))

# Listen for screenshot (blocking) and return it
async def get_screenshot(websocket, discord_channel):
    global EXIT
    while not EXIT:
        screenshot = await websocket.recv()
        await send_screenshot(screenshot, discord_channel)
        #print(screenshot) #DEBUG
        print("[" + time.strftime("%H:%M:%S", time.localtime()) + "] Got a screenshot.")

async def start(host, port, discord_channel):
    # Start websocket client
    print(f"Starting on ws://{host}:{port}")
    uri = f"ws://{host}:{port}"
    async with websockets.connect(uri) as websocket:
        await get_screenshot(websocket, discord_channel)

@client.event
async def on_ready():
    print(f"{client.user} has connected to Discord!")

@client.event
async def on_guild_join(guild):
    await guild.system_channel.send(f"Use `vitto help` to get help.")

@client.event
async def on_error(message):
    await message.channel.send("The bot encountered an unexpected error and needs to be restared.")

# Messages handling
# Send message: `await message.channel.send("...")`
# Fetch message: `message.content`
@client.event
async def on_message(message):
    if message.author.id == client.user.id:
        return

    # Start
    if message.content.startswith("vitto start"):

        # Use defaults if it's empty
        host = DEFAULT_HOST
        port = DEFAULT_PORT
        try:
            host = message.content.split(" ")[2]
            port = message.content.split(" ")[3]
        except: # Either host or port are
            try:
                host = message.content.split(" ")[2]
                print("No port specified. Using default port...")
            except:
                print("No host or port specified. Using defaults...")
        
        # Start the program based on the IP/hostname after "start"
        await start(host, port, message.channel)

    # Stop
    elif message.content.lower() == "vitto stop":
        global EXIT
        EXIT = True
    # Easter egg
    elif "sono " in message.content:
        msg = message.content
        str_to_find = "sono "

        # Index of the first character of str_to_find in msg
        index = msg.find(str_to_find)

        # Check if there's a negation
        if msg.find("non sono ") == index - 4:
            return
        
        # Delete the first part of the message which doesn't contain "sono ", including "sono "
        printable_msg = msg[index + len(str_to_find):]

        # String truncation caused by one of the endings
        end = -1

        # When these are found, the printable string ends
        endings = [",", "."]

        # Used to exit all loops
        found = False

        for i in range(len(printable_msg)):
            if not found:
                for e in endings:
                    if printable_msg[i] == e:
                        end = i
                        found = True

        if end != -1:
            printable_msg = printable_msg[:end]
        
        await message.channel.send(f"Ciao { printable_msg.strip() }, sono Vittorio.")

    # Help
    elif message.content.lower() == "vitto help":
        await message.channel.send(f"Use `vitto start <host> <port>` to receive screenshots from the backend's host address and port.\n \
        Defaults: {DEFAULT_HOST} and {DEFAULT_PORT} (`{DEFAULT_ADDRESS}`).\n \
        Use `vitto stop` to stop receiving screenshots.")


client.run(TOKEN)
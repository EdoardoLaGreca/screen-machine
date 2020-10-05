#
# DOCS:
#  - [Discord]    https://discordpy.readthedocs.io/en/latest/api.html
#  - [PIL]        https://pillow.readthedocs.io/en/stable/reference/
#

import discord # `pip install discord.py`
import time
import os
from PIL import Image
import socket
import sys

from dotenv import load_dotenv # `pip install python-dotenv`

load_dotenv()
TOKEN = os.getenv('DISCORD_TOKEN')

client = discord.Client()

BUFFER_SIZE = 4096
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = "4444"
DEFAULT_ADDRESS = f"ws://{DEFAULT_HOST}:{DEFAULT_PORT}"
EXIT = False

# Send screenshot to client
async def send_screenshot(data, discord_channel):
    print(len(data))
    # Separate the height data from the screenshot data by the separator '|'
    # data = data.split(bytes('|', 'utf-8'), 1)
    # recvd_height = data[0].decode('utf-8')
    # print("Received height (from bytes) = " + str(data[0]))
    # print("Image length = " + str(len(data[1])))

    # height = int(recvd_height)

    # if height != 0:
    #     img_data = data[1]

    #     width = int((len(img_data) / 3) / height)
    #     img = Image.frombuffer("RGB", (width, height), img_data, "raw")
    #     await discord_channel.send(discord.File(img))
    # else:
    #     print("Height is wrong (zero).")


# async def collect_data(data, discord_channel):
#     for fragment in data:

# From: https://stackoverflow.com/a/22207830
# def recvall(sock, count):
#     buf = b''
#     while count:
#         newbuf = sock.recv(count)
#         if not newbuf: return None
#         buf += newbuf
#         count -= len(newbuf)
#     return buf

# Message: b'<img_height>|<img_bytes>|<img_data...>'
async def tcp_handler(socket, discord_channel):
    global EXIT

    # For all connections
    while not EXIT:

        print("Waiting for a screenshot...")

        # Where the image is
        img_data = b''

        # Initial bytes to read
        initial_bytes = 50

        # Image data size in bytes
        img_bytes = 0

        initial_buffer = socket.recv(initial_bytes)

        # Data from the initial buffer
        initial_data = initial_buffer.split(bytes('|', 'utf-8'), 2)

        # Metadata (img_height + img_bytes)
        metadata = initial_data[0:1]
        metadata_bytes = len(''.join(map(str, metadata))) + 2

        # Metadata parsed
        img_height = int(metadata[0].decode('utf-8'))
        img_bytes = int(metadata[1].decode('utf-8'))


        img_data += initial_data[2]

        # Total message bytes (metadata + img_bytes)
        #total_bytes = metadata_bytes + img_bytes
        
        # Read the bytes of image that haven't been read yet
        while img_bytes - len(img_data) > 0:
            img_data += socket.recv(BUFFER_SIZE)
            
        print("[" + time.strftime("%H:%M:%S", time.localtime()) + "] Got a screenshot.")

        await send_screenshot(img_data, discord_channel)
        print("Screenshot sent to Discord!")


# Listen for screenshot (blocking) and return it
async def start(host, port, discord_channel):
    global EXIT
    print(f"Connecting to {host}:{port}")
    
    # Create a TCP/IP socket
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

    # Connect the socket to the port where the server is listening
    s.connect((host, int(port)))
    await tcp_handler(s, discord_channel)


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
        #asyncio.get_event_loop().run_until_complete(start(host, port, message.channel))
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

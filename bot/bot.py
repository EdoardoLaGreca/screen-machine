#
# DOCS:
#  - [Discord]    https://discordpy.readthedocs.io/en/latest/api.html
#  - [PIL]        https://pillow.readthedocs.io/en/stable/reference/
#

# TODO:
# - fix the workaround in send_screenshot()

import discord # `pip install discord.py`
import aiohttp # Included with discord.py
import time
import os
from PIL import Image
from io import BytesIO
import socket
import sys

from dotenv import load_dotenv # `pip install python-dotenv`

load_dotenv()
TOKEN = os.getenv('DISCORD_TOKEN')

client = discord.Client()

SLEEP_TIME_EMPTY_MSG = 1.0 # seconds
BUFFER_SIZE = 4096
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = "4040"
DEFAULT_ADDRESS = f"ws://{DEFAULT_HOST}:{DEFAULT_PORT}"
EXIT = False

# Send screenshot to client
async def send_screenshot(data: bytes, height: int, discord_channel: discord.channel.TextChannel):

    data_length = len(data)

    if height != 0 and data_length != 0:

        width = int((data_length / 3) / height)
        print("  Image size = " + str(width) + "x" + str(height) +", Image total length = " + str(data_length))

        # Workaround: Save the file as PNG, load it and send it. BytesIO does not work
        image = Image.frombytes("RGB", (width, height), data)
        image.save("screenshot.png")
        #attachment = discord.File(BytesIO(image.tobytes()))
        attachment = discord.File("screenshot.png")
        await discord_channel.send(file=attachment)
    else:
        print("Cannot send screenshot: invalid data")


def get_img_data(length: int, initial_data: bytes, socket: socket.socket):
    # The image is stored here (as bytes)
    img_data = b''

    img_data += initial_data

    # Read the bytes of image that haven't been read yet
    while len(img_data) < length:
        img_data += socket.recv(BUFFER_SIZE)

    return img_data

# Returns:
#     bool -> was the request successful? (if it wasn't: bytes is empty and int is 0)
#     bytes -> image as bytes
#     int -> image height
def request_image(socket: socket.socket) -> (bool, bytes, int):
    # Request the image
    socket.send(b"more")

    # Wait to receive data
    img_data = b''

    # Initial bytes to read to parse the metadata (img_height and img_bytes)
    initial_bytes = 50

    # Image data size in bytes
    img_bytes = 0

    initial_buffer = socket.recv(initial_bytes)

    # Data from the initial buffer
    initial_data = initial_buffer.split(bytes('|', 'utf-8'), 2)

    # If it can correctly be parsed, it means that a new screenshot has been received. Otherwise just retry.
    if len(initial_data) == 3:

        print("[" + time.strftime("%H:%M:%S", time.localtime()) + "] Got a screenshot.")

        # Metadata (img_height + img_bytes)
        metadata = initial_data[0:2]

        # Parsed metadata
        img_height = int(metadata[0].decode('utf-8'))
        img_bytes = int(metadata[1].decode('utf-8'))

        img_data = get_img_data(img_bytes, initial_data[2], socket)
        return (True, img_data, img_height)
    elif len(initial_data) == 1 or len(initial_buffer) == 0:
        print("No data received")
        return (False, b'', 0)

# Message: b'<img_height>|<img_bytes>|<img_data...>'
async def tcp_handler(socket: socket.socket, discord_channel: discord.channel.TextChannel):
    global EXIT

    while not EXIT:
        
        #token = ""

        print("Waiting for a screenshot...")

        success, img_data, img_height = request_image(socket)

        if success:
            await send_screenshot(img_data, img_height, discord_channel)
            print("  Screenshot sent to Discord!")

            time.sleep(SLEEP_TIME_EMPTY_MSG)
        # Couldn't be parsed
        else:
            return


# Listen for screenshot (blocking) and return it
async def start(host: str, port: str, discord_channel: discord.channel.TextChannel):
    global EXIT

    while not EXIT:

        # Create a TCP/IP socket
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

        print(f"Connecting to {host}:{port}...")
        # Connect the socket to the port where the server is listening
        s.connect((host, int(port)))
        print("  Connected.")
        
        await tcp_handler(s, discord_channel)
        s.close()
        time.sleep(SLEEP_TIME_EMPTY_MSG)


@client.event
async def on_ready():
    print(f"{client.user} has connected to Discord!")


@client.event
async def on_guild_join(guild):
    await guild.system_channel.send(f"Use `vitto help` to get help.")


# @client.event
# async def on_error(message: discord.message.Message):
#     await message.channel.send("The bot encountered an unexpected error and needs to be restared.")


# Messages handling
# Send message: `await message.channel.send("...")`
# Fetch message: `message.content`
@client.event
async def on_message(message: discord.message.Message):
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
    
    elif "grazie vittorio" in message.content.lower() or "grazie, vittorio" in message.content.lower():
        await message.channel.send("Figurati figliolo")

    # Help
    elif message.content.lower() == "vitto help":
        await message.channel.send(f"Use `vitto start <host> <port>` to receive screenshots from the backend's host address and port.\n \
        Defaults: {DEFAULT_HOST} and {DEFAULT_PORT} (`{DEFAULT_ADDRESS}`).\n \
        Use `vitto stop` to stop receiving screenshots.")


client.run(TOKEN)

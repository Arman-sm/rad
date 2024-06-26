
<p align=center>
  <img src="design/icon.png" width="250"/>
</p>

# Rad
Rad is a server that can mix multiple tracks and play them across multiple devices in sync.

## Project Status
The project is still incomplete, untested, and under heavy development so it may undergo breaking changes.

## Supported Platforms
The program has only been tested on Linux ( Ubuntu 24.4 ) but I may start testing it on Windows soon as well.
The program should probably work on Macs, but it won't be tested since I don't have a Mac.

## Usage
The program can be compiled and run using cargo without extra dependencies.

For now, the program may only be controlled via the debug cli which comes right after the program initialization.
The available commands and their way of use can be found in the debug cli itself and with the use of the `help` command.

But generally, in this program, there are some components that you should probably know before using it

For the program to work you must give it some arguments:
| Short | Long | Description | Default |
| :---: | :--: | :---------: | :-----: |
| -D | --enable-dbg-cli | Enable the debug cli | true (temporarily) |
| -d | --data-dir | Sets the directory in which the program will be saving its data in | Windows: %PROGRAMDATA%\rad\data\ \| UNIX: /var/lib/rad/ |
| -c | --config | Path to the configuration file | Windows: %PROGRAMDATA%\rad\rad.conf \| UNIX: /etc/rad/rad.conf |

## Concepts
These concepts exist throughout the project and some are even accessible and are interacted with directly using the debug cli
Note: Concepts marked with `DEV` only need to be known if you're working with the source code.

### Source (DEV)
A source is a generic representation of an audio stream.
Each source has a function inside of it that returns the next part of the stream.

This function is unique to the source and has to be given to the source on initialization.
There usually are helper functions that create this function and the source for you, as an example for reading an audio file you can call `rad_compositor::sources::symphonia::init_symphonia_src` and pass a file to it.

### Composition
Compositions are configuration, audio, and arrangement data for Compositors to produce audio with, they're the only way in which other sections of the program can control the produced audio.
Examples of things stored in compositions that are changeable through the debug cli:
- Sources
- Time
- Amplification
- Pause/Play State

### Compositor (DEV)
The compositor is a part of the server that mixes audio coming from the sources.
Each compositor instance is run on a separate thread and calculates frames ahead of the playback. 
There can be multiple compositors, even for the same composition as each compositor computes frames for a specific sample rate.

### Adapter
Adapters are a generic way to manage different outputs. (Closing, Fetching their status, etc.)

The reason they exist is that when playing audio to different devices, each device may be limited to communication to one method or two and so the communication can't be done through a single protocol so managing all the variations can get complex over time.

## What does RAD stand for?
It stands for Rust Audio Daemon.


<p align=center>
  <img src="design/icon.png" width="250"/>
</p>

# Rad
Rad is an audio daemon which can compose multiple audio streams and play them across multiple devices in sync.

## Project Status
The project is still incomplete, untested, and under heavy development so it may undergo breaking changes.

## Supported Platforms
The program has only been tested on GNU/Linux (Specifically recent versions of Ubuntu) but I may start testing it on Windows soon as well.
The program should probably work on Macs, but it won't be tested since I don't have a Mac.

## Usage
The program can be compiled and run using cargo without extra dependencies.

By default the program is controllable using the REST API provided on the address (`api_addr`) provided in the configuration file but you may also enable the dbg cli and use that instead.

The available commands in the dbg cli and their way of use can be found in the debug cli itself and with the use of the `help` command.

### Command line arguments
| Short | Long | Description | Default |
| :---: | :--: | :---------: | :-----: |
| -D | --enable-dbg-cli | Enable the debug cli | false |
| -d | --data-dir | Sets the directory in which the program will be saving its data in | Windows: %PROGRAMDATA%\rad\data\ \| UNIX: /var/lib/rad/ |
| -c | --config | Path to the configuration file | Windows: %PROGRAMDATA%\rad\rad.conf \| UNIX: /etc/rad/rad.conf |
| -l | --log-level | Program log level (trace, debug, warn, error) | debug: debug \| release: warn |

## Concepts
In order to use the program or contribute to it, there are some concepts we suggest you take a look at:


These concepts exist throughout the project and some are even accessible and are interacted with directly using the debug cli

* Concepts marked with `DEV` only need to be known if you're working with the source code.

### Source (DEV)
A source is a generic representation of an audio stream whether it's a live audio or static audio loaded in memory.

### Composition
Compositions are configuration, audio, and arrangement data for `Compositors` to produce audio with, they're the only way in which other sections of the program can control the produced audio.

Examples of things stored in compositions that are changeable through the debug cli:
- Sources
- Time
- Amplification
- Pause/Play State

### Compositor (DEV)
A compositor is a component of the program that composes streams of audio coming from sources in the arrangement and with the configuration set for it.

Some of the configurations set for a compositor is changeable throughout the lifespan of the compositor and on the fly like time, sources, and amplification all stored on a composition and there are also some settings that the compositor is initialized with and aren't changeable so a new one must be made if one wanted to change them like the sample rate.

Each compositor instance is run on a separate thread and produces audio ahead of the playback in buffers stored as nodes of a linked list like structure and constantly updates a reference to a node of the produced audio in the composition registry so that output endpoints can provide the audio.

There can be multiple compositors, even for the same composition as each compositor produces audio for a specific sample rate.

### Adapter
Adapters are a generic way to manage different outputs. (Closing, Fetching their status, etc.)

The reason they exist is mostly about the ability to easily implement new adapters for different jobs as there isn't a single universally agreed interface and protocol available on all devices and suitable for every network to use the produced audio and it would be very difficult to develop a fit for all, as one may decide to use udp and the other may prefer http, one might want to use a specific compression algorithm and one might want to use it for playing audio on their discord server and one might even want to use bluetooth!

## What does RAD stand for?
It stands for Rust Audio Daemon.

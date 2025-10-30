# Rust Photo Booth

A production-ready photo booth application built in Rust for Raspberry Pi, featuring Canon DSLR camera control, automated printing, and web-based interface.

## Background

I created this photo booth as a project for friends who run a board game company and needed an interactive booth for conventions. The goal was to build a reliable, self-contained system that could capture high-quality photos, add custom branding/templates, and print on-site without requiring constant supervision.
I was partly successful, but had a few issues that showed up on the days of the event.
The main issue was the camera interation, I have too much going on in the capture/ path, and on top of that the camera used at the event was different enough from my setup I was building with that the timing was really off on the camera capture.
The second issue was most likely with the printer setup. We used a new type of printer, but for some reason it didn't show errors on it's screen and I had to check it with the lp commands for cups to find the error and relay that to the team, during the event. Also I had to manually re-enable the printer in cups after the errors, which I never had to do before the event with other types of printers. 
All of that being said I think it was generally successful, and people had fun with it, so that's a win.

### Disclaimer
There's a good bit of AI slop in here, I really tried to find a workflow where I passed the agent a task while I worked on other parts, but it always ended up leaving things I didn't want or didn't like around, and the only reason there are still traces of that, is I haven't gone through and cleaned it up yet. Also pretty much all of the frontend was initially AI and then I adjusted, it's definitely a weakness of mine, so I'm sure the flat html files with js and css in them isn't ideal, but it works and actually looks pretty nice. The major issue I have is I had to manually tweak the sizing to get it to work on the screen at the coference, because for some reason (even though we explicitely planned for this and got screens with matching resolutions and quality) the sizing was totally different on mine at home, than on the one they had at the conference, so I just had to send it and fiddle with sizing with a healthy dose of hoping AI could actually "make the buttons fit better, they look weird". 

### Why I Chose Rust

After experimenting with several languages and approaches:

- **Python**: [Python Repo](https://github.com/georgehyde-dot/PiPhotoBooth) This was my initial prototype to get something on a screen using all Raspberry Pi built in utilities and a simple frontend. I started here becasue I found a lot of photobooths built in python, and it seemed like an easy way to go from 0 to 1, and get a general feel for what the project would require. The initial prototype was too slow for real-time camera preview and had reliability issues with long-running processes. The first issue I ran into (due to some Python skill issues on my part) was that objects were getting cleaned up by the GC when I didn't want them to be. I reworked the structure to create an initial set up that lived for the life of the program, and then an object per iteration of a user going through the photobooth. Also, at this point, I was using a Raspberry Pi camera V2, which had a package I could use for easy set up. As I started to look more into how it worked, I knew I would want some lower level control, and that led to me going to my next choice of C++.
- **C++**: [C++ Repo](https://github.com/georgehyde-dot/BantamPhotoBoothQtVersion) Initially here I looked at doing Rust or C++, but the initial set up to start working on the Pi was very simple with C++, and I made a poor attempt at cross compiling my Rust, that led to me shelving Rust. I was also interested in doing a larger project in C++, and I thought it would be easy to do my dev directly on the Pi in C/C++. I got neovim set up, and started doing research on how to display the frontend. I think my choice of display was really my downfall here. I chose QT, initially not thinking much of the GPL license requirement. I also didn't think about how I would eventually shift to working with a group of people on the designs, none of whom had much coding experience, let alone familiarity with QT frontends. In general I liked my structure for the flow of screens, but I wasn't quite able to meet my goal of having a simple initial memory allocation, and then a single allocation per iteration, due to the built in QT memory model. I ended up spending a large chunk of time looking into memory issues related to how I was using the QT objects, and I began to regret my choices. Also, I was talking with my friends about the frontend, and it quickly became obvious that I needed to use a more web based frontend to integrate their designs. Of all of my options, this is the one that I want to go back to the most, because I think the control over the camera would have been the most straightforward with the best libraries (gphoto2). At this point I started looking at Rust and Go
- **Go**: For a brief moment I looked at using Go for the project. I use it constantly as an SRE in my day job, and I am much more proficient in it than my other options. That being said, I didn't like the existing Go packages for CUPS and Gphoto that I found. They were very old, and I expected I would end up doing a lot of the work myself, or relying on CLI calls, which, at this point, I wanted to avoid. That left me with Rust.

I settled on Rust for several key reasons:
- **Memory Management**: I could avoid the issues I ran into with C++ and Python, while having access to C projects if necessary (I ended up scrapping that due to time constraints, but its in the plan for the future)
- **Systems control**: I knew if I dug deep enough I could control the camera stream cleanly and efficiently
- **Reliability**: The type system catches many issues at compile time, so I could rely on fewer backwards breaking changes(This ended up not being true, but I was optimistic to start)
- **Cross-compilation**: Once I set up the docker build pipeline for the target aarch64 system, I had no issues building and deploying. Realistically this wasn't a bonus over the other options, because for C++ I was working directly on the pi, and the Go build for different targets is incredibly easy, but it's a decent bullet point I guess. 
- **Ecosystem**: Lots of resources for web servers (I initially started with axum, then switched to actix-web), and the templating system was very easy for the final image as well. The templating was the easiest of the languages I tried, and initially I was looking to, From Zero to Production in Rust a bit, but as time got more tight I moved away from that rigor.

## Development

### Development Journey


#### Coordination:
Really the longest chunk of time was going back and forth figuring out what was needed for the project and what the actual flow through the screens would be. 
#### Preview System
Implementing real-time preview was challenging. This is one of the major issues still.
- **v4l2loopback**: Virtual video device for streaming
- **gphoto2**: Captures live view from Canon camera
- **FFmpeg**: Pipes the stream to the loopback device
- **MJPEG streaming**: Serves preview to web browsers

#### Printer Integration
Really just relying on CUPs and the packages that exist for that.
#### Remote Deployment
After I got tailscale set up on the devices it became trivial to scp files and manually fiddle with things over ssh.
#### Frontend Evolution
This went through a lot of phases, and I really like what we ended up with. Getting the text just right for each of the options was something I spent a lot of time with, and I think it fit in the world of Bantam pretty well.
### Technical Highlights

#### v4l2loopback Setup
I realized that I couldn't stream from the camera and capture images as the same timeout without making one of a few compromises:
- Just streaming the data from the screen and doing a capture of a still image from the stream
   -- This had an added issue of the capture showing the focus window, and I built a pretty neat algorithm to remove the box as a post process on the capture, but I had issues with the screen timing out, so this one was a bust.
- Buying a new camera, realistically and for future attempts this is exactly what I'll do, but I was already a few hundred dollars in on the camera, and I wanted to make it work.
- Get lucky and find a stack overflow comment describing how to create a loopback device and pass it the camera stream through ffmpeg.
   -- This turned out being what I went with, and then I would cancel the stream, and use gphoto2 to perform the actual capture, then on the start of the next iteration, start the stream back.
```bash
sudo modprobe v4l2loopback devices=1 video_nr=10 card_label="Photo Booth" exclusive_caps=1

gphoto2 --stdout --capture-movie | ffmpeg -i - -vcodec rawvideo -pix_fmt yuv420p -f v4l2 /dev/video10
```

## Deployment

### Deployment System Overview

The deployment system uses Docker for cross-compilation and SSH for distribution:

1. **Build Phase**: Docker container cross-compiles for ARM64
2. **Distribution**: Binary and assets deployed via SCP
3. **Configuration**: Environment-based configuration for different venues
4. **Monitoring**: Tailscale for remote access and troubleshooting. I just used tail on the log file since it was just me monitoring it and I was pretty familiar with all the logs. 

### Docker Build System

The Dockerfile implements a multi-stage build:
```dockerfile
# Build stage: Debian Bookworm with cross-compilation toolchain
FROM debian:bookworm AS builder
# Install Rust and ARM64 cross-compilation tools
# Build for aarch64-unknown-linux-gnu target
# Output minimal binary artifact
```

### Deployment Script

The `deploy.sh` script handles the complete deployment:

```bash
./deploy.sh [environment] [deploy_all]
# Examples:
./deploy.sh dev        # Deploy to development Pi
./deploy.sh prod       # Deploy to production Pi
./deploy.sh dev true   # Deploy with all setup scripts
```

Features:
- Environment-specific deployment (dev/prod) This split between deploying to the actual pi in use at the conference or the one sitting next to me in my office. Just an IP switch.
- Intelligent file change detection (only copies modified files) The wifi was bad at the conference so I needed to keep my push size down.

### Setup Scripts

The deployment includes several setup scripts:
Each of these was a response to some specific issue I ran into while I was setting everything up and testing it locally. I think by the end they weren't actually needed, but I'm glad that I have them as a check on how I got things done and some of the issues I ran into.

- **setup_packages.sh**: Installs system dependencies (gphoto2, ffmpeg, CUPS, etc.)
- **setup_printer.sh**: Configures CUPS and TurboPrint driver
- **configure_printer_4x6.sh**: Sets printer defaults for photo printing
- **check_setup.sh**: Diagnostic script for troubleshooting
- **install_fonts.sh**: Installs custom fonts for template rendering

### Troubleshooting

Here are some things I ran into pretty often/during the event

#### Camera Issues
- **Device Busy**: Increase delay after stopping preview, I never really got this fixed. Major issues
- **Preview Frozen**: Restart v4l2loopback module. This was generally only an issue while I was trying to fix the timing and I caused some side issues with the stream.

#### Printer Issues
- **Jobs Stuck**: Check CUPS queue with `lpstat -o`
- **Wrong Size/Wrong printing settings**: I ran into a conflict between what I sent to the printer and its default settings a few times and got the wrong paper size out. The fix was to stick with one driver from the start and set the printers default to match what I was sending in the request to cups.

#### Deployment Issues
- The main issue I ran into was that my deployment required that the service be turned off during the deploy. This was a rookie mistake on my part, but other issues kept me from fixing this one. 

### Monitoring & Maintenance

Production monitoring setup:
- Tailscale for secure remote access
- SystemD service for automatic startup
- Log rotation for long-running deployments
- I should have set up data backups to run regularly during the event, and to compress the images, but it fell out as I had other more pressing priorities. 

## Project Structure

```
canon_test_cam/
├── src/
│   ├── main.rs              # Application entry point
│   ├── gphoto_camera.rs     # Canon camera control
│   ├── routes/              # HTTP endpoint handlers
│   ├── templates.rs         # Print template generation
│   ├── config.rs            # Configuration management
│   └── printers/            # Printer abstraction
├── migrations/              # Database schema migrations
├── static/                  # Frontend assets
├── deploy.sh               # Deployment script
├── Dockerfile              # Cross-compilation container
└── operations/             # Setup and maintenance scripts
```

## Acknowledgments

Special thanks to:
- The Bantam team for trusting me with their convention booth needs, their product is vastly superior to what I created, so don't hold my mediocre code and project against their amazing company. 
Really go check them out [Bantam Planet](https://www.visitbantam.com/)

## License

MIT License - See LICENSE file for details

## Some pictures from the development
A view of one of the earlier development setups I had
![photo_2025-10-29 19 06 16](https://github.com/user-attachments/assets/559c0a70-13d8-4b18-a7d2-bcebaa4c6da8)

A quick design diagram of the system
![photo_2025-10-29 19 06 36](https://github.com/user-attachments/assets/f0b8f535-4e93-46af-89c4-7b3cec714d91)

I moved halfway through development and this was my setup for a week or so in the middle of an empty room
![photo_2025-10-29 19 07 37](https://github.com/user-attachments/assets/510be12f-6bc9-48ec-a865-a51e21459e0c)

One of the earlier captures before I switched from the picam to a DSLR
![photo_2025-10-29 19 12 06](https://github.com/user-attachments/assets/23f9d56c-32d1-4b0b-9bb6-9f7805d3f813)

Image-ception
![20250917_122245](https://github.com/user-attachments/assets/d64b49a0-5268-4263-a3ab-35d9d466f2bc)

One of the first wanted posters
![20250819_124323](https://github.com/user-attachments/assets/149a194a-a422-4086-bea8-f27d5c471153)

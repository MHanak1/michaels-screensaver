<div align="center">

  <h1><code>Michael's Screensaver</code></h1>

<strong>A (growing) collection of screensavers made by yours truly.</strong>
  <p>
    <a href="https://mhanak.net/screensaver">Live Demo</a> | <a href="https://github.com/MHanak1/michaels-screensaver/releases"> Download </a>
  </p>
</div>

## About
A screensaver app written in Rust, using [wgpu](https://wgpu.rs/) for rendering, and [egui](https://egui.rs) for the config GUI 

Currently, it consists of 2 screensavers:

 * **Snow** - A couple of hills, with snow slowly falling.
 * **Balls** - Balls Bouncing off of each other and off the screen sides. Highly configurable, with different color modes and presets. Turns out, that this is also a pretty decent gas simulation (since the balls follow the same rules as gas particles)
## Usage
### Any Ol' Web Browser*
* Go to https://mhanak.net/screensaver
* Enjoy

\*Any browser\*\* that supports WebGL

\*\*I know WebGL doesn't work on Chromium on Linux by default
### Windows
* Download the `michaels-screensaver.scr` file from the Releases section on the right
* Once downloaded, right-click the file -> Install
* Done!
* If you want to configure it, either do it through settings, or right-click -> Configure
### Linux
#### To Play With It
* In the directory you have downloaded the binary run:
  * `./michaels-screensaver` - to run it
  * `./michaels-screensaver --help` - for the list of commands (also tells you where the config file is located)
  * `./michaels-screensaver --config` -for the config GUI
#### To Use as an Actual Screensaver
* ¯\\\_(ツ)\_/¯
* Try [this script](https://askubuntu.com/questions/707855/how-to-execute-a-command-after-a-certain-period-of-inactivity-triggered-by-keyb) (I may eventually build that into the screensaver)
### macOS
* does not and probably never will not work as an actual screensaver. as far as I know MacOS handles screensavers differently, and even if I did figure out how to do that, I do not have a device I could test it on.
## Building
### Native
`cargo run --release` - the compiled binary *should* be somewhere in the `target/release` folder
### Web Assembly
`wasm-pack build --target web --release` - the generated folder `pkg` together with `index.html` and `index.css` are needed for the web version.
`./serve.py` (or `python serve.py`) - run it locally (with cache disabled)


to do both on linux run `if wasm-pack build --target web --release; then ./serve.py; fi`
## Configuration
Other Than the configuration GUI, you can configure the screensaver in a couple ways
### Config File (native)
the config file is located in `C:\Users\UserName\AppData\Roaming\michaels-screensaver.toml` on Windows, or `~/.config/michaels-screensaver.toml` on Linux
```toml
# contents of default_config.toml

#avaliable screensavers: snow, balls
screensaver = "balls"
fullscreen = true

[snow]
snowflake_count = 7500

[balls]
speed = 0.1
count = 10000
size = 0.05
#random - a random color, it changes when balls bounce off of each other
#color - a flat color.
#temperature - makes it so the color's hue depends on a given particle's velocity. may impact perfromance
#infection - one ball is chosen, it has a different color to every other ball and it is infected. ever ball that touches an infected ball becomes infected itself. after all balls get infected a new one is chosen
color_mode = "infection"
color = "#ff00ff"
#makes it so the opacity of a ball is dependent on the ammount of balls in the surrounding regions. if the region size is lower, the contrast will be higher
show_density = true
#used for the density color mode. the determines what density is considered "high"
target_display_density = 10.0
#for optimisation's sake, the space is divided up into regions, of which the size depends on the ball size, and this value. increase this value for spare simulations, decrease it for dense ones
#do not decrease it under 0.5, otherwise the simulation will start glitching
#if you want to learn more look up "spatial hashing"
region_size = 1.0
#whether the balls should slow down/speed up if the average speed is higher/lower than the configured speed.
correct_ball_velocity = true
```
### URL Parameters (web)
The parameters are converted into a TOML and then loaded as standard config, because of that every value visible above can be changed through the url (some of them, like `fullscreen` are ignored in the web version). the `screensaver` parameter should always be before all the screensaver options.

The parameters should be structured like so:

`https://example.com/?screensaver=screensaver_name&parameter1=value&parameter2=value`
 
so for example
`https://mhanak.net/screensaver/?screensaver=balls&size=0.1&color_mode=color&color=%23ff0055&show_density=false`
gets turned into such TOML file:
```toml
screensaver = "balls"
[balls] # this gets added because of the previous line.
size = 0.1
color_mode="color"
color = "#ff0055" # '#' was encoded using  %23
show_density = false
```

keep in mind that as long as this project is in development, there is a chance that the way it handles URL parameters may change, breaking created links in the process.
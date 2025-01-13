<div align="center">

  <h1><code>Michael's Screensaver</code></h1>

<strong>A (growing) collection of screensavers made by yours truly.</strong>
  <p>
    <a href="https://mhanak.net/screensaver">Live Demo</a>
  </p>
</div>

## About
A screensaver app written in Rust, using [wgpu](https://wgpu.rs/) for rendering, and [egui](https://egui.rs) for the config GUI 
## Usage
### Any Ol' Web Browser*
* Go to https://mhanak.net/screensaver
* Enjoy

\*Any browser\*\* that supports WebGL

\*\*I know WebGL doesn't work on Chromium on Linux by default
### Windows
* Download the `michaels-screensaver.scr` file
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
* Try [this script](https://askubuntu.com/questions/707855/how-to-execute-a-command-after-a-certain-period-of-inactivity-triggered-by-keyb) (i may eventually build that into the screensaver)
### MacOS
* Probably doesn't work. (but if you have a mac and really need to run this natively, feel free to reach out to me)
## Building
### Native
`cargo run --release` - the compiled binary *should* be somewhere in the `target/release` folder
### Web Assembly
`wasm-pack build --target web --release` - the generated folder `pkg` together with `index.html` and `index.css` are needed for the web version.
`./serve.py` (or `python serve.py`) - run it locally (with cache disabled)
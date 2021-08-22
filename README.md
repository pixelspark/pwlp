# Pixelspark Wireless Led Protocol (PWLP) server

## Building

### Regular build
````sh
cargo build
````

### For Raspberry Pi
````sh
cargo build --target=arm-unknown-linux-musleabi --features=raspberrypi 
````

To streamline building and uploading to a Raspberry Pi, use [build_pi.sh](./build_pi.sh). Add the following to your SSH 
config file (~/.ssh/config):

````
Host rpi
	HostName raspberrypi.local
	User pi
	IdentityFile ~/.ssh/id_rsa
	StrictHostKeyChecking no
	UserKnownHostsFile /dev/null
````

After adding your SSH public key (`~/.ssh/id_rsa.pub`) to `~/.ssh/authorized_keys` on the Pi, you will be able to upload 
without using a password. If you don't have an SSH key yet, run `ssh-keygen`.

### WASM

````sh
cargo install wasm-pack
wasm-pack build --target=web --release -- --no-default-features --features=wasm
````

See [index.html](./index.html) for a usage example. To test:

````sh
npm install -g http-server
http-server
````

### Programs

The binaries will include several default programs as binaries; these are in the [src/programs](./src/programs) folder
and can be rebuilt using `./generate_programs.sh`:

* `off.{txt, bin}`: the program that is sent to strips to turn off
* `default_serve.{txt, bin}`: the default program served when no other program is specified in settings/command line

## Usage

````
# Compile a script
cargo run -- compile test/random.txt test/random.bin

# Test run a script
cat test/random.txt | cargo run -- run

# Serve programs to devices (configure using config.toml)
cargo run -- serve

# Run a client (configure using config.toml)
cargo run -- client

# Run a program
cargo run -- run --binary test/clock.bin

# Run a program on an actual strip with 100 LEDs (SPI bus 0 SS 0) on a Raspberry
cargo run -- run --binary --hardware -l 100 test/clock.bin

# Run a program on an actual strip with 100 LEDs connected to SPI bus 1 slave select 1 on a Raspberry
cargo run -- run --binary --hardware --bus 1 --ss 1 -l 100 test/clock.bin
````

## Protocol

The PLWP protocol devices a message format as well as an instruction architecture. Scripts are compiled to this architecture and transmitted using the message format to the devices, who will execute them.

For more information see [protocol.md](https://git.pixelspark.nl/pixelspark/espled/src/branch/master/Protocol.md).

## Script language

Example scripts can be found in the [test](./test/) folder. 

### Statements

Consecutive statements are separated by ";". Supported constructs:

* `if(comparison) { statements }`
* `loop { statements }`: loops `statements` forever
* `for(var=expression) { statements }`: counts `var` down from `expression` to 1 (inclusive), e.g. `for(n=5)` will loop with n=5, 4, 3, 2, 1.
* Comments and whitespace:
  * `/* may span multiple lines */`
  * `// single line` (should end in `\n`)
  * `\r`, `\n`, `\t` and ` ` are whitespace
* Special commands:
  * `yield`
* User commands:
  * `get_pixel(index)`: gets the current value for a pixel (may not be blitted yet); formatted as 0x00BBGGRR
  * `set_pixel(i, r, g, b)`: set pixel at index `i` to color `(r, g, b)`
  * `random(max)`: return a random number between zero and `max`, inclusive
  * `get_length`: returns the length of the strip
  * `get_precise_time`: returns a monotonic time in milliseconds. In deterministic mode, uses the number of instructions to return an approximate time.
  * `get_wall_time`: returns the number of seconds elapsed since the Unix epoch time (possibly wrapping around in the future!).
* Compiler intrinsics:
  * `rgb(r, g, b)` translates to `(r & 0xFF) | (g & 0xFF) << 8 | (b & 0xFF) << 16`
  * `red(c)` translates to `c & 0xFF`
  * `green(c)` translates to `(c >> 8) & 0xFF`
  * `blue(c)` translates to `(c >> 16) & 0xFF`

### Expressions

Supported operators:

* Arithmetic: `a+b`, `a/b`, `a*b`, `a-b`, `a%b`
* Binary: `a|b`, `a&b`, `a^b` (XOR)
* Unary: `!a`
* Comparison: `a==b`, `a!=b`, `a<b`, `a>b`, `a<=b`, `a>=b`

## API

### GET `/`

Get server status. Can be used for health checking.

````json
{}
````

### GET `/devices`

Returns a list of devices currently or previously connected.

````json
{
  "devices": {
    "aa-bb-cc-dd-ee-ff": {
      "address": "1.2.3.4:5678",
      "program": [10, 11, 12, ...]
    }
  }
}
````

### GET `/devices/<mac>`

Returns information on a specific device

````json
{
  "address": "1.2.3.4:5678",
  "program": [10, 11, 12, ...]
}
````

### GET `/devices/<mac>/<program_name>`

Send a built-in program to the device. Built-in program names:

* [`off`](./src/programs/off.txt)
* [`default`](./src/programs/default_serve.txt)

````json
{}
````

## License

[MIT](./LICENSE.txt)
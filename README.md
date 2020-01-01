# Pixelspark Wireless Led Protocol (PWLP) server

## Building

Regular build:
````sh
cargo build
````

For Raspberry Pi:
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

## Usage

````
# Compile a script
cargo run -- compile test/random.txt test/random.bin

# Test run a script
cat test/random.txt | cargo run -- run

# Serve programs to devices (configure using config.toml)
cargo run -- serve

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

Example scripts can be found in the [tests](./tests/) folder. 

### Statements

Consecutive statements are separated by ";". Supported constructs:

* `if(comparison) { statements }`
* `loop { statements }`: loops `statements` forever
* `for(var=expression) { statements }`: counts `var` down from `expression` to zero (inclusive).
* Special commands:
  * `yield`
* User commands:
  * `set_pixel(expression)`
  * `get_pixel(index)`
  * `set_pixel(i, r, g, b)`: equivalent to `set_pixel(i | r<<8 | g<<16 | b<<24)`
  * `random(expression)`: return a random number between zero and `expression`, inclusive
  * `get_length`: the length of the strip
  * `get_precise_time`
  * `get_wall_time`
* Compiler intrinsics:
  * `irgb(i, r, g, b)` translates to `(i & 0xFF) | (r & 0xFF) << 8 | (g & 0xFF) << 16 | (b & 0xFF) << 24`
  * `red(c)` translates to `(c >> 8) & 0xFF`
  * `green(c)` translates to `(c >> 16) & 0xFF`
  * `blue(c)` translates to `(c >> 24) & 0xFF`
  * `index(c)` translates to `c & 0xFF`

### Expressions

Supported operators:

* Arithmetic: `a+b`, `a/b`, `a*b`, `a-b`, `a%b`
* Binary: `a|b`, `a&b`, `a^b` (XOR)
* Unary: `!a`
* Comparison: `a==b`, `a!=b`, `a<b`, `a>b`, `a<=b`, `a>=b`
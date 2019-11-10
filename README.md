# Pixelspark Wireless Led Protocol (PWLP) server

## Usage

````
# Compile a script
cargo run -- compile test/random.txt test/random.bin

# Test run a script
cat test/random.txt | cargo run -- run

# Serve programs to devices (configure using config.toml)
cargo run -- serve
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
  * `set_pixel(i, r, g, b)`: equivalent to `set_pixel(i | r<<8 | g<<16 | b<<24)`
  * `random(expression)`: return a random number between zero and `expression`, inclusive
  * `get_length`: the length of the strip
  * `get_precise_time`
  * `get_wall_time`

### Expressions

Supported operators:

* Arithmetic: `a+b`, `a/b`, `a*b`, `a-b`, `a%b`
* Binary: `a|b`, `a&b`, `a^b` (XOR)
* Unary: `!a`
* Comparison: `a==b`, `a!=b`, `a<b`, `a>b`, `a<=b`, `a>=b`
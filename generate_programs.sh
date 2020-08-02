#!/bin/sh

for i in ./src/programs/*.txt; do
	cargo run -- compile $i ${i%.*}.bin
done
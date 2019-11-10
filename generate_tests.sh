#!/bin/sh

for i in ./test/*.txt; do
	cargo run -- compile $i ${i%.*}.bin
	cargo run -- disassemble ${i%.*}.bin > ${i%.*}.dis
done
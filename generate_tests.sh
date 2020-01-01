#!/bin/sh

for i in ./test/*.txt; do
	cargo run -- compile $i ${i%.*}.bin
	cargo run -- disassemble ${i%.*}.bin > ${i%.*}.dis
	cargo run -- run $i --instruction-limit 2560 --length 10 --deterministic > ${i%.*}.out
done
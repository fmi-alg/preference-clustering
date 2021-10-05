To build:
	make

print-ilp expects the output of inst_gen as input and prints the result

example:
	./build/print-ilp < ../inst_gen/build/Sets.out

To save the result to a file run

	./build/print-ilp < ../inst_gen/build/Sets.out > ilp.txt

To solve the ilp run
	
	glpsol --lp ilp.txt --output solution.txt


greedy-solver takes Sets.out and lp.sol as input

example:

	glpsol --lp ilp.txt --output ilp-lponly.sol --nomip
	
	./build/greedy-solver Sets.out ilp-lponly.sol

The naive greedy solver takes only a sets file as input

    ./build/naive-greedy-solver Sets.out

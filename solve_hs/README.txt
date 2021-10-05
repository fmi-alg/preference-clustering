To build:
	make

print-ilp expects the output of inst_gen as input and prints the result

example:
	./print-ilp < ../inst_gen/build/Sets.out

To save the result to a file run

	./print-ilp < ../inst_gen/build/Sets.out > ilp.txt

To solve the ilp run
	
	glpsol --lp ilp.txt --output solution.txt


greedy-solver takes Sets.out and lp.sol as input

example:

	glpsol --lp ilp.txt --output ilp-lponly.sol --nomip
	
	./greedy-solver Sets.out ilp-lponly.sol

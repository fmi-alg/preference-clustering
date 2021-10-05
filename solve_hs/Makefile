CC := g++

all: greedy-solver print-ilp naive-greedy-solver

build:
	mkdir -p build

%: %.cpp build
	${CC} -O3 $< -o build/$@

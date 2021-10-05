To build:

	mkdir build
	cd build
	cmake ../ -DCMAKE_BUILD_TYPE="Release"
	make -j 4


For a sanity check there is simple data set "test_inst.txt" which can also be seen in "test_inst.pdf".

	./arrang ../test_inst.txt
runs the Set generator on the data set and outputs a file Sets.out which contains one line per vertex p. of the arrangement:

<p.x()> <p.y() <polygonID containing p> <polygonID containing p> ....

Things are not tuned at all at the moment, but there is a lot of tuning potential.


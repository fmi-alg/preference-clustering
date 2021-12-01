# Hitting Set Generator

This part of the project creates hitting set instances from a set of preference spaces. It contains three executables.

`arrang` takes a .space file and creates the arrangement of all preference polyhedra that are contained. Based on this, it outputs all the vertices of the arrangement in two ways. First as a .sets file which contains the information which preference spaces are covered by this vertex and as a .pts files which has the coordinates of the vertex.

`SetMinimizer` takes a .sets file and removes all dominiated sets from it.

`contained` is a helper that takes at least two .space files (for example outer approximation, exactly computed and inner approximation) and checks that for each preference space in a file the preference spaces with the same index in the later files are contained within the former.

# Build

    mkdir build
    cd build
    cmake ../ -DCMAKE_BUILD_TYPE="Release"
    make -j 4

For a sanity check there is simple data set "test_inst.txt" which can also be seen in "test_inst.pdf".

    ./arrang ../test_inst.txt

runs the Set generator on the data set and outputs a file Sets.out which contains one line per vertex p. of the arrangement:

<p.x()> <p.y() <polygonID containing p> <polygonID containing p> ....

Things are not tuned at all at the moment, but there is a lot of tuning potential.

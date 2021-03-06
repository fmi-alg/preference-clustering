# Hitting Set Generator

This part of the project creates hitting set instances from a set of preference spaces. It contains three executables.

`arrang` takes a .space file and creates the arrangement of all preference polyhedra that are contained. Based on this, it outputs all the vertices of the arrangement in two ways. First as a .sets file which contains the information which preference spaces are covered by this vertex and as a .pts files which has the coordinates of the vertex.

`SetMinimizer` takes a .sets file and outputs a new .sets file with all dominated sets removed.

`contained` is a helper that takes at least two .space files (for example outer approximation, exactly computed and inner approximation) and checks that for each preference space in a file the preference spaces with the same index in the later files are contained within the former.

# Build

    mkdir build
    cd build
    cmake ../ -DCMAKE_BUILD_TYPE="Release"
    make -j 4

For a sanity check there is simple data set "test_inst.txt" which can also be seen in "test_inst.pdf".

    ./arrang ../test_inst.txt

For a visual representation the "-w" flag can be added **after** the input file. This will show a window with the polyhedra and their intersections.

    ./arrang ../input.txt -w

# Output Files

The Set generator `arrang` outputs a file Sets.out which contains one line per vertex p of the arrangement:

    <polygonID containing p> <polygonID containing p>

It also outputs a file Sets.pts which contains one line per vertex p with coordinates of p:

    <p.x()> <p.y()>

# Creating Preference Polyhedra

This part of the project creates preference polyhedra for trajectories.
For this, it uses the two approaches from our paper.

- Exact Polyhedron Construction via the Corner Cutting Approach
- Approximate inner and outer Polyhedron Construction via uniform sampling

# Build

The application is written in rust and can be compiled with cargo.

```sh
cargo build --release
```

# Usage Examples

## Approximate Preference Spaces

For creating approximate polyhedra the `random_approx_instances` executable is used.

```sh
./target/release/random_approx_instances --help
pref-polys 0.1.0

USAGE:
    random_approx_instances [FLAGS] [OPTIONS] [graph]

FLAGS:
        --config-only    Exit after writing config file
    -h, --help           Prints help information
    -V, --version        Prints version information

OPTIONS:
    -a, --approx <approx>                Approximation Strategy to use. Possible values are: axis, random, rotation
    -c, --approx-count <approx-count>    Changes the number of directions to approximate in (only applicable with random
                                         and rotation)
        --config-file <config-file>
    -p, --num-paths <num-paths>
    -n, --num-prefs <num-prefs>
    -o, --output-path <output-path>       [default: .]
    -f, --paths-file <paths-file>
    -s, --seed <seed>

ARGS:
    <graph>    Path to the Graphfile
```

As an input it needs a graph file, an approximation strategy and the
trajectories to work on. The trajectories can be specified via a paths file in
yml format or via the parameters to generate them randomly. Randomly generated
trajectories are written to `paths.yml` in the output directory.

## Exact Preference Spaces

For creating approximate polyhedra the `exact_preference_areas` executable is used.

```sh
./target/release/exact_preference_areas --help
pref-polys 0.1.0
USAGE:
    exact_preference_areas [OPTIONS] <graph>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --debug-output <debug-output>
    -m, --modus <modus>                  Used mode:
                                           - 0: generate 'p' trajectories with 'n' different preferences,
                                                output preference spaces (default)
                                           - 1: generate 1 trajectory, output area representation
                                           - 2: generate 'p' trajectories, output most complex area representation
                                           - 3: read paths file, output preference spaces
    -p, --num-paths <num-paths>          Amount of trajectories to generate
    -n, --num-prefs <num-prefs>          Amount of preferences to use when generating trajectories
    -o, --output <output>                Path to output file
    -f, --path-file <path-file>          Path to paths files
    -s, --seed <seed>                    seed for randomly generated trajectories

ARGS:
    <graph>    Path to the graph file
```

Its most important option, apart from the graph file, is the modus. This value
determines what the application does and which other cli options are used. The
most important values are 0 and 3. Both write the exact preference areas into a
.space file. The first uses the `num-prefs` and `num-paths` value to generate paths,
while the second uses trajectories read from the `path-file`.

# Used File Formats

## Graph Files

Graph files for this project can have several comment lines at the beginning of
the file indicated by a leading "#". Those are followed by 3 meta data lines.
The first contains the size of the costs vectors. The second the number of nodes
and the third the number of edges in the graph.

Afterwards all the nodes are listed line by line with the following information
separated by space:

- node id
- ch level

This is followed bey all edges line by line with following information separated
by space:

- source node id
- target node id
- cost value 1
- cost value 2
- ...
- cost value n
- skipped edge id 1 (if edge is a shortcut, -1 otherwise)
- skipped edge id 2 (if edge is a shortcut, -1 otherwise)

Cost values are required to be integers.

## Space files

The output of this project are the preference space polyhedra of the
trajectories. Therefore all the vertex coordinates lie in the intervall [0,1].
To be able to represent those vertices exactly, we represent them by the
constraints that intersect in the vertex. The file is structured as follows.
In the first line the amount of trajectories/preference spaces is listed. In
each subsequent line, you will find first the number of vertices of the space
and then 6\* #vertices integers which are the coefficients a,b,c of the constraints in
the form ax + by + c = 0.

## Paths file

The paths file is a simple yaml file that lists all node ids, edge ids and the costs
of the trajectories used. A sample trajectory from node 0 to node 2 via edge 0 looks like this:

```yaml
- nodes:
    - 0
    - 2
  edges:
    - 0
  total_dimension_costs:
    - 3000.0
    - 8000.0
    - 50000.0
```

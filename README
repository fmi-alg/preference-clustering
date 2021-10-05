# Preference-based Trajectory Clustering

This repository contains the software that belongs to the paper
"Preference-based Trajectory Clustering -- An Application of Geometric Hitting
Sets" which will be published in the proceedings to ISAAC 2021. The application
takes a set of trajectories (e.g. routes driven by one or more drivers) and
finds a set of preferences such that any trajectory is optimal for at least one
preference under the personalized route planning model (as stated
[here](https://doi.org/10.1145/2820783.2820830)). To do this we create geometric
hitting set instances out of the preference polyhedra of the trajectories and
solve them.

# Building

There are three different code bases in this repository. With the `build.sh`
script in the root directory all binaries will be built. A list of dependencies
and detailed build instruction for the different parts is included in their
individual READMEs.

# Experiments

The Experiments made for the ISAAC paper are all based on randomly generated
trajectories. You can setup new experiments with the `setup-experiment.sh`
script. It will create a folder containing a config and a make file to run the experiment.

# Experiment Results

Our paper presents two approaches to create the geometric hitting set instances.
One is to create an inner and outer approximation of the preference polyhedra
so that the solution to the inner instance is feasible while the solution to the
outer instance is a lower bound to the optimum. The other approach computes the
polyhedra exactly. Therefore files in the experiment folder are prefixed with
inner, outer and exact depending on the approach and end with their suffix for
their contents. The following suffixes are possible:

```
 .space  <- Contains the perefernce polyhedra
 .init_sets.pts <- contains all intersection points between polyhedra
 .init_sets <- specifies for each point which polyhedra contain it
 .sets  <- like .init_sets but unnecessary points were removed
 .lp   <- ILP formulation of the hitting set problem
 .lpsol <- Solution of the LP-Relaxiation
 .ilpsol <- Solution by the ILP solver
 .naivegreedysol <- Solution by the naive greedy solver
 .greedysol <- Solution by randomized greedy solver
 .times <- Timings of each step
```

Also the these files are created:

```
paths.yml <- Created trajectories
results.txt <- Textual summary of the experiment results
spaces.containment_check <- Sanity check that for each polyhedron: <img src="https://render.githubusercontent.com/render/math?math=\text{inner}%20\subseteq%20\text{exact}%20\subseteq%20\text{outer}">
```

#include <iostream>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>
#include <assert.h>
#include <math.h>
#include <algorithm>
#include "Timer.h"

using namespace std;

typedef vector<int> Set;
typedef vector<int> Path;

int main(int argc, char *argv[])
{
    cout << "Usage: " << argv[0] << " Sets.out lp.sol [seed]" << endl;
    assert(argc >= 3);
    if(argc > 3){
        uint seed = stoul(argv[3]);
        srand(seed);
        cout << "Usind seed " << seed << endl;
    }
    int num_paths;
    int num_sets;
    int num_sets_with_activity = 0;
    int lower_bound = -1;
    vector<double> activities;
    {
        double objective_value = 0;
        ifstream reader(argv[2]);
        string line;
        for (int i = 0; i < 9; i++)
        {
            getline(reader, line);
            istringstream iss(line);
            if (i == 1)
            {
                string tmp;
                iss >> tmp;
                iss >> num_paths;
            }
            else if (i == 2)
            {
                string tmp;
                iss >> tmp;
                iss >> num_sets;
            }
            else if (i == 5)
            {
                string tmp;
                iss >> tmp;
                iss >> tmp;
                iss >> tmp;
                iss >> objective_value;
            }
        }
        for (int i = 0; i < num_paths + 3; i++)
        {
            getline(reader, line);
        }
        double sum_activity = 0;
        for (int i = 0; i < num_sets; i++)
        {
            getline(reader, line);
            istringstream iss(line);
            for (int j = 0; j < 3; j++)
            {
                string tmp;
                iss >> tmp;
            }
            double activity;
            iss >> activity;
            activities.push_back(activity);
            sum_activity += activity;
            if(activity > 0){
                num_sets_with_activity++;
            }
        }
        if(abs(sum_activity - objective_value) >= 0.001) {
            cout << "Error: " << sum_activity << " " << objective_value << " " << abs(sum_activity - objective_value) << endl;
        }
        assert(abs(sum_activity - objective_value) < 0.001);
        lower_bound = ceil(sum_activity);
    }
    vector<Set> sets;
    vector<Path> paths;
    string line;
    {
        ifstream reader(argv[1]);
        string line;
        while (getline(reader, line))
        {
            istringstream iss(line);
            int path_index;
            Set set;
            while (iss >> path_index)
            {
                if (path_index + 1 > paths.size())
                {
                    paths.resize(path_index + 1);
                }
                set.push_back(path_index);
                paths.at(path_index).push_back(sets.size());
            }
            sets.push_back(set);
        }
        assert(num_paths == paths.size());
        assert(num_sets == sets.size());
    }
    Timer timer;
    vector<int> best_solution;
    int counter = 0;
    while (counter < paths.size() && (counter == 0 || best_solution.size() > lower_bound))
    {
        timer.cont();
        vector<int> count_picked(paths.size(), 0);
        int num_uncovered = paths.size();
        vector<int> solution;
        vector<int> indices(paths.size(), 0);
        vector<int> permutation(paths.size());
        for (int i = 0; i != indices.size(); i++)
        {
            indices[i] = i;
            permutation[i] = rand();
        }
        sort(indices.begin(), indices.end(),
             [&](const int &a, const int &b) {
                 return (permutation.at(a) > permutation.at(b));
             });
        for (int i = 0; i < paths.size(); i++)
        {
            int index = indices.at(i);
            if (count_picked.at(index) > 0)
            {
                continue;
            }
            Path &path = paths.at(index);
            bool finished = false;
            int set_index = -1;
            while (!finished)
            {
                set_index = path.at(rand() % path.size());
                double token = ((double)rand() / (RAND_MAX));
                if (token < activities.at(set_index)){
                    finished = true;
                }
            }
            solution.push_back(set_index);
            for (int p_id : sets.at(set_index))
            {
                if (count_picked.at(p_id) == 0)
                {
                    num_uncovered--;
                }
                count_picked.at(p_id)++;
            }
            if(num_uncovered == 0){
                break;
            }
        }
        assert(num_uncovered == 0);
        vector<int> pruned_solution;
        vector<int> set_indices(solution.size(), 0);
        vector<int> set_permutation(solution.size());
        for (int i = 0; i != solution.size(); i++)
        {
            set_indices[i] = i;
            set_permutation[i] = rand();
        }
        sort(set_indices.begin(), set_indices.end(),
             [&](const int &a, const int &b) {
                 return (set_permutation.at(a) > set_permutation.at(b));
             });
        for (int i = 0; i < solution.size(); i++)
        {
            int set_id = solution.at(set_indices.at(i));
            bool can_be_pruned = true;
            for (int j = 0; j < sets.at(set_id).size(); j++)
            {
                if (count_picked.at(sets.at(set_id).at(j)) <= 1)
                {
                    if (count_picked.at(sets.at(set_id).at(j)) <= 0)
                    {
                        cout << "Error: count picked should be > 0 but is " << count_picked.at(sets.at(set_id).at(j)) << endl;
                    }
                    can_be_pruned = false;
                    break;
                }
            }
            if (can_be_pruned)
            {
                for (int j = 0; j < sets.at(set_id).size(); j++)
                {
                    count_picked.at(sets.at(set_id).at(j))--;
                }
            }
            else
            {
                pruned_solution.push_back(set_id);
            }
        }
        if (best_solution.size() == 0 || best_solution.size() > pruned_solution.size())
        {
            best_solution = pruned_solution;
            cout << "best solution after " << counter+1 << " rounds: " << best_solution.size() << " (lower bound is " << lower_bound << ")" << endl;
        }
        counter++;
        timer.stop();
    }
    cout << "Best solution after " << counter << " rounds: " << best_solution.size() << endl;
    cout << "Lower bound: " << lower_bound << endl;
    cout << "cover:";
    for (int i = 0; i < best_solution.size(); i++)
    {
        cout << " " << best_solution.at(i);
    }
    cout << endl;

    return 0;
}
#include <iostream>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>
#include <assert.h>
#include "Timer.h"

using namespace std;

typedef pair<double, double> Intersection;
typedef vector<uint> Set;
typedef vector<uint> Path;

int main(int argc, char *argv[])
{
    vector<Set> sets;
    vector<Path> paths;
    string line;
    while (getline(cin, line))
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
    Timer timer;
    vector<int> solution;
    vector<int> cover_counter(sets.size(), 0);
    for (int i = 0; i < sets.size(); i++)
    {
        cover_counter.at(i) = sets.at(i).size();
    }
    int num_uncovered = paths.size();
    vector<int> count_picked(paths.size(), 0);
    while (num_uncovered > 0)
    {
        int max_cover = 0;
        int set_index = -1;
        for (int i = 0; i < sets.size(); i++)
        {
            if (cover_counter.at(i) > max_cover)
            {
                max_cover = cover_counter.at(i);
                set_index = i;
            }
        }
        solution.push_back(set_index);
        num_uncovered -= cover_counter.at(set_index);
        for (int path : sets.at(set_index))
        {
            if (count_picked.at(path) == 0)
            {
                for (int set : paths.at(path))
                {
                    cover_counter.at(set)--;
                }
            }
            count_picked.at(path)++;
        }
        if (cover_counter.at(set_index) != 0)
        {
            cout << "Error: cover counter is " << cover_counter.at(set_index) << endl;
        }
        assert(cover_counter.at(set_index) == 0);
    }
    vector<int> pruned_solution;
    for (int i = 0; i < solution.size(); i++)
    {
        int set_id = solution.at(i);
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
    cout << "solution: " << pruned_solution.size() << endl;
    cout << "cover:";
    for (int i = 0; i < pruned_solution.size(); i++)
    {
        cout << " " << pruned_solution.at(i);
    }
    cout << endl;
    return 0;
}
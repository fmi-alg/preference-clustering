#include <assert.h>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <vector>

using namespace std;

typedef vector<uint> Path;

int main(int argc, char *argv[]) {
  size_t intersections = 0;
  vector<Path> paths;
  string line;
  int num_paths = 0;
  while (getline(cin, line)) {
    istringstream iss(line);
    uint path_index;
    while (iss >> path_index) {
      if (path_index + 1 > num_paths) {
        num_paths = path_index + 1;
        paths.resize(num_paths);
      }
      paths.at(path_index).push_back(intersections);
    }
    intersections += 1;
  }
  cout << "Minimize" << endl;
  cout << "\tValues: ";
  for (int i = 0; i < intersections; i++) {
    cout << "x" << i + 1;
    if (i < intersections - 1) {
      cout << " + ";
    }
  }
  cout << endl << endl;
  cout << "Subject To" << endl;
  for (int i = 0; i < paths.size(); i++) {
    if (paths[i].size() == 0) {
      cerr << "Error: path " << i << " is not covered by any preference!"
           << endl;
      exit(1);
    }
    cout << "c" << i + 1 << ":\t";
    for (int j = 0; j < paths[i].size(); j++) {
      cout << "x" << paths[i][j] + 1;
      if (j < paths[i].size() - 1) {
        cout << " + ";
      }
    }
    cout << " >= 1" << endl;
  }
  cout << endl;
  cout << "Binary" << endl;
  for (int i = 0; i < intersections; i++) {
    cout << "\t"
         << "x" << i + 1 << endl;
  }
  cout << "End" << endl;
  return 0;
}

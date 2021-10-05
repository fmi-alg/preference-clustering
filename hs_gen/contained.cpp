#include "maxCL.h"

#include <iostream>

void release_assert(bool condition, std::string text) {
  if (!condition) {
    std::cout << text << '\n';
    exit(1);
  }
}

int main(int argc, char *argv[]) {

  if (argc <= 2) {
    std::cout << "USAGE: " << '\n'
              << argv[0] << " file1 file2 [files..]" << '\n';
    std::cout
        << "Files should be ordered by contained polygon size (large -> small)"
        << '\n';
    exit(1);
  }

  std::vector<std::vector<Polygon_2>> polyLists;

  for (int i = 1; i < argc; i++) {
    std::vector<Polygon_2> polys{};
    readPolysFromIS(argv[i], polys);
    polyLists.push_back(polys);
  }

  for (int i = 1; i < polyLists.size(); i++) {
    release_assert(polyLists[i - 1].size() == polyLists[i].size(),
                   "Not all inputs have the same amount of polygons");
  }

  std::cout << "checking containement from outside to inside" << '\n';

  bool wrong = false;
  for (int j = 0; j < polyLists[0].size(); j++) {
    for (int i = 1; i < polyLists.size(); i++) {
      auto &outer = polyLists[i - 1][j];
      auto &inner = polyLists[i][j];

      for (auto vertex = inner.vertices_begin(); vertex != inner.vertices_end();
           ++vertex) {
        if (outer.has_on_unbounded_side(*vertex)) {
          std::cout << "Polygon " << j << " of file " << argv[i]
                    << " does not contain point " << *vertex
                    << " of the polygon from file " << argv[i + 1] << '\n';
          wrong = true;
          break;
        }
      }
    }
  }

  if (wrong) {
    exit(1);
  }

  for (int i = 0; i < polyLists.size(); i++) {
    size_t complexity = 0;
    for (int j = 0; j < polyLists[i].size(); j++) {
      complexity += polyLists[i][j].size();
    }
    double avg = (double)complexity / (double)polyLists[i].size();
    std::cout << argv[i + 1] << ": average polygon complexity: " << avg << '\n';
  }

  std::cout << "outer files contain inner polygons as expected" << '\n';

  return 0;
}

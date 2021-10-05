#ifndef MAXCL_H
#define MXCL_H

#include <CGAL/Arr_polyline_traits_2.h>
#include <CGAL/Arr_segment_traits_2.h>
#include <CGAL/Arrangement_2.h>
#include <CGAL/Exact_predicates_exact_constructions_kernel.h>
#include <CGAL/Polygon_2.h>
#include <CGAL/draw_polygon_2.h>
#include <fstream>
#include <list>
#include <vector>

using namespace std;

typedef CGAL::Exact_predicates_exact_constructions_kernel K;
typedef CGAL::Polygon_2<K> Polygon_2;
typedef CGAL::Point_2<K> Point;
typedef CGAL::Line_2<K> Line;
typedef Polygon_2::Edge_const_iterator EdgeIterator;
typedef Polygon_2::Vertex_iterator VertexIterator;

typedef CGAL::Arr_segment_traits_2<K> Segment_traits_2;
typedef CGAL::Arr_polyline_traits_2<Segment_traits_2> Geom_traits_2;
typedef Geom_traits_2::Segment_2 Segment_2;
typedef Geom_traits_2::Curve_2 Polyline_2;
typedef CGAL::Arrangement_2<Geom_traits_2> Arrangement_2;

void constructArrang(vector<Polygon_2> &myPolys, Arrangement_2 &arr,
                     Geom_traits_2 &traits) {

  Geom_traits_2::Construct_curve_2 polyline_construct =
      traits.construct_curve_2_object();

  // iterate over all polygons and add them as polylines to the arrangement
  for (int i = 0; i < myPolys.size(); i++) {
    Polygon_2 curPoly = myPolys[i];
    if (curPoly.size() > 1) {
      for (EdgeIterator ei = curPoly.edges_begin(); ei != curPoly.edges_end();
           ++ei) {
        // std::cout << "edge to insert is " << *ei << std::endl;
        Segment_2 curSeg(*ei);
        insert(arr, curSeg);
      }
    } else {
      cout << "0-dimensional (ignored for arrangement): " << curPoly[0] << endl;
    }
  }
}

void readPolysFromIS(char *fname, vector<Polygon_2> &myPolys) {
  ifstream myFile(fname);
  int nofPolys;
  myFile >> nofPolys;
  cout << "Reading " << nofPolys << " polygons" << endl;
  for (int i = 0; i < nofPolys; i++) {
    int nofCorners;
    Polygon_2 curPoly;
    myFile >> nofCorners;
    //    cout << endl << "Polygon with " << nofCorners << " corners: ";
    for (int j = 0; j < nofCorners; j++) {
      int a, b, c, d, e, f;
      myFile >> a >> b >> c >> d >> e >> f;
      Line l1(a, b, c), l2(d, e, f);

      CGAL::Object result = CGAL::intersection(l1, l2);

      Point ipoint;
      Line iline;
      if (CGAL::assign(ipoint, result)) {
        if (!curPoly.is_empty() &&
            (curPoly.vertex(curPoly.size() - 1) == ipoint ||
             curPoly.vertex(0) == ipoint)) {
          // cout << "duplicate detected in poly " << i << " point " << j << " "
          //      << ipoint << endl;
        } else
          curPoly.push_back(ipoint);
        //      cout << " " << ipoint << "\t";
      } else if (CGAL::assign(iline, result)) {
        cout << "Error: Lines " << l1 << " and " << l2 << " are identical."
             << endl;
        exit(1);
      } else {
        cout << "Error: Lines " << l1 << " and " << l2 << " do not intersect."
             << endl;
        exit(1);
      }
    }
    //    cout << endl;
    if (!curPoly.is_simple()) {
      cout << "Polygon " << i << " is not simple. aborting" << endl;
      exit(1);
    }
    if (!curPoly.is_convex()) {
      cout << "Polygon " << i << " is not convex. aborting" << endl;
      exit(1);
    }
    myPolys.push_back(curPoly);
  }
}
#endif

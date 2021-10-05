#include "maxCL.h"
#include <CGAL/Bbox_2.h>
#include <CGAL/Interval_nt.h>
#include <CGAL/Qt/GraphicsViewNavigation.h>
#include <CGAL/Real_timer.h>
#include <CGAL/box_intersection_d.h>
#include <CGAL/intersections.h>
#include <QApplication>
#include <QGraphicsScene>
#include <QGraphicsView>
#include <QLineF>
#include <QRectF>
#include <QtGui>
#include <boost/format.hpp>
#include <iostream>
#include <omp.h>

typedef CGAL::Box_intersection_d::Box_d<double, 2> Box;

// callback function object writing results to an output iterator
template <class OutputIterator> struct Report {
  OutputIterator it;
  Report(OutputIterator i) : it(i) {} // store iterator in object
  // We write the id-number of box a to the output iterator assuming
  // that box b (the query box) is not interesting in the result.
  void operator()(const Box &a, const Box &b) {
    *it++ = a.id();
    *it++ = b.id();
  }
};
template <class Iter> // helper function to create the function object
Report<Iter> report(Iter it) {
  return Report<Iter>(it);
}

int main(int argc, char **argv) {
  bool showWindow = false;
  if (argc > 2 && strcmp(argv[2], "-w") == 0) {
    showWindow = true;
  }
  QApplication *app = nullptr;
  if (showWindow)
    app = new QApplication(argc, argv);

  ofstream outFile("Sets.out");
  ofstream outFilePts("Sets.pts");
  ifstream myFile(argv[1]);
  vector<Polygon_2> myPolys;
  CGAL::Real_timer myTimer;
  myTimer.start();

  readPolysFromIS(argv[1], myPolys);

  size_t nofPolys = myPolys.size();
  cout << "Reading finished at " << myTimer.time() << " with " << nofPolys
       << " polygons" << endl;

  /*
  for(size_t i=0; i<nofPolys; i++)
  {
          cout<<"Poly "<<i<<" has "<<myPolys[i].size()<<endl;
  }
*/
  myTimer.start();
  Geom_traits_2 traits;
  Arrangement_2 arr(&traits);
  constructArrang(myPolys, arr, traits);
  cout << "After full insertion at " << myTimer.time() << endl;

#define SIZE 500

#define SCALE 500

  QGraphicsScene *scene = nullptr;
  if (showWindow) {
    scene = new QGraphicsScene();
    scene->setSceneRect(0, 0, SIZE, SIZE);
    scene->addRect(QRectF(0, 0, SIZE, SIZE));
  }

  typename Arrangement_2::Edge_const_iterator eit;

  std::cout << arr.number_of_edges() << " edges:" << std::endl;
  for (eit = arr.edges_begin(); eit != arr.edges_end(); ++eit) {
    Point src = eit->source()->point();
    Point trg = eit->target()->point();
    CGAL::Interval_nt<true> srcX, srcY, trgX, trgY;
    srcX = (src.x()).interval();
    srcY = src.y().interval();
    trgX = (trg.x()).interval();
    trgY = trg.y().interval();
    // cout<<"Edge: "<<src<<" "<<trg<<endl;
    if (showWindow)
      scene->addLine(QLineF(SCALE * srcX.sup(), SIZE - SCALE * srcY.sup(),
                            SCALE * trgX.sup(), SIZE - SCALE * trgY.sup()));
  }

  QGraphicsView *view = nullptr;
  if (showWindow) {
    view = new QGraphicsView(scene);
    CGAL::Qt::GraphicsViewNavigation navigation;
    view->installEventFilter(&navigation);
    view->viewport()->installEventFilter(&navigation);
    view->setRenderHint(QPainter::Antialiasing);
    view->show();
  }

  cout << "After scene init " << myTimer.time() << endl;

  typename Arrangement_2::Vertex_const_iterator vit;

  cout << "We have " << arr.number_of_vertices() << " vertices" << endl;

  int count_sings = 0;
  for (int i = 0; i < nofPolys; i++) {
    if (myPolys[i].size() == 1) {
      Point curPt = myPolys[i][0];
      outFilePts << curPt << "\n";
      for (int j = 0; j < nofPolys; j++) {
        Polygon_2 &curPoly = myPolys[j];
        if (curPoly.size() == 1) {
          if (curPt == myPolys[j][0]) {
            outFile << j << " ";
            count_sings++;
          }
        } else {
          int orient = curPoly.bounded_side(curPt);
          if ((orient == CGAL::ON_BOUNDED_SIDE) ||
              (orient == CGAL::ON_BOUNDARY)) {
            outFile << j << " ";
            count_sings++;
          }
        }
      }
      outFile << endl;
    }
  }
  cout << "Found " << count_sings << " singular point sets" << endl;
  // put polygons in data structure
  vector<Box> myBoxes;
  for (int i = 0; i < nofPolys; i++) {
    CGAL::Bbox_2 curBB = myPolys[i].bbox();
    myBoxes.push_back(curBB);
  }
  cout << "FF: After Box collection " << myBoxes.size() << " " << myTimer.time()
       << endl;

  // collect all vertices
  vector<Box> myVertices;
  vector<Point> myPoints;
  for (vit = arr.vertices_begin(); vit != arr.vertices_end(); ++vit) {
    myVertices.push_back((vit->point()).bbox());
    myPoints.push_back(vit->point());
  }
  cout << "FF: After vertex collection " << myVertices.size() << endl;

  std::vector<std::size_t> result;
  CGAL::box_intersection_d(myBoxes.begin(), myBoxes.end(), myVertices.begin(),
                           myVertices.end(),
                           report(std::back_inserter(result)));
  cout << "FF: Found " << result.size() / 2 << " box candidates"
       << " " << myTimer.time() << endl;

  vector<vector<long>> collectedSets;
  collectedSets.resize(myVertices.size(), vector<long>());
  int ff_count_sets = 0;
  for (int i = 0; i < result.size(); i += 2) {
    //	   cout<<"Checking "<<result[i]<<" and "<<result[i+1]<<endl;
    Polygon_2 &curPoly = myPolys[result[i]];
    Point curPt = myPoints[result[i + 1] - myPolys.size()];
    assert(curPoly.size() > 1);
    int orient = curPoly.bounded_side(curPt);
    if ((orient == CGAL::ON_BOUNDED_SIDE) || (orient == CGAL::ON_BOUNDARY)) {
      ff_count_sets++;
      collectedSets[result[i + 1] - myPolys.size()].push_back(result[i]);
    }
  }
  for (int i = 0; i < collectedSets.size(); i++) {
    sort(collectedSets[i].begin(), collectedSets[i].end());
    for (int j = 0; j < collectedSets[i].size(); j++)
      outFile << collectedSets[i][j] << " ";
    outFile << endl;
  }

  cout << "FF: After computation of " << ff_count_sets << " associations at "
       << myTimer.time() << endl;

  // only draw if not too many things
  int nofVerts = myVertices.size();

  for (vit = arr.vertices_begin(); vit != arr.vertices_end(); ++vit) {
    // cout<<"Vertex to consider: "<<vit->point()<<endl<<"is contained in: ";
    CGAL::Interval_nt<true> xCoord, yCoord;
    xCoord = (vit->point()).x().interval();
    yCoord = (vit->point()).y().interval();
    if (showWindow && nofVerts < 1000)
      scene->addEllipse(xCoord.sup() * SCALE - 5,
                        SIZE - yCoord.sup() * SCALE - 5, 10, 10);
    Point curPt = vit->point();
    outFilePts << curPt << "\n";
    /*
    // #pragma omp parallel for
    for (int i = 0; i < nofPolys; i++) {
      int orient = myPolys[i].bounded_side(curPt);
      if ((orient == CGAL::ON_BOUNDED_SIDE) || (orient == CGAL::ON_BOUNDARY)) {
        //			    cout<<i<<" ";
        nofAssoc++;
        // outFile << i << " ";
      }
    }
    //    cout<<endl;
    // outFile << endl;
   */
  }
  /*
  for (int i = 0; i < nofProcs; i++)
    nofAssoc += assocs[i];
  cout << nofAssoc << " associations (non singular) after " << myTimer.time()
       << endl;
*/
  outFile.close();
  outFilePts.close();
  cout << "Right before exiting" << endl;
  if (nofVerts < 1000) {
    if (showWindow)
      return app->exec();
  }
  return 0;
}

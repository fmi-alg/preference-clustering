#include<iostream>
#include<assert.h>
#include<vector>
#include<fstream>
#include<string>
#include<sstream>
#include<algorithm>
#include<cmath>
#include <omp.h>


using namespace std;

class MSet
{
public:
	vector<int> items;

	MSet(const vector<int>&init)
	{
		items=init;
	}
	void printSet(ostream &os)
	{
		for(int j=0; j<items.size(); j++)
			os<<items[j]<<" ";
		os<<endl;
	}
	bool operator != (const MSet& mset) const
	{
		if (items.size()!=mset.items.size())
			return true;
		for(int i=0; i<items.size(); i++)
			if (items[i]!=mset.items[i])
				return true;
		return false;
	}

	bool operator < (const MSet& mset) const
    	{
        	if (items.size()< mset.items.size()) return true;
        	if (items.size()> mset.items.size()) return false;
		// we have same size now
		assert(items.size()==mset.items.size());
		for(int i=0; i<items.size(); i++)
			if (items[i]<mset.items[i])
				return true;
			else if (items[i]>mset.items[i])
				return false;
		return false;
    	}
	bool isDominatedBy(const MSet&mset) const
	{
		int posMe=0, posYou=0;
		if (mset.items.size()<items.size())
			return false;
		while (posMe<items.size()&&(posYou<mset.items.size()))
		{
			if (items[posMe]==mset.items[posYou])
			{
				posMe++; posYou++;
			}
			else if (items[posMe]<mset.items[posYou])
				return false;
			else if (items[posMe]>mset.items[posYou])
				posYou++;
		}
		if (posMe==items.size())
			return true;
		else
			return false;
	}
};

int main(int argc, char *argv[])
{
	vector<MSet> mySets;
	vector<bool> domSet;

	ifstream inFile(argv[1]);
	string curLine;
	while(getline(inFile,curLine))
	{
		if (!inFile.eof())
		{
			vector<int> curSet;
			istringstream iss(curLine);
			do
			{
				int it;
				iss>>it;
				if (!iss.fail())
				  curSet.push_back(it);
			} while (!iss.eof());
			mySets.push_back(curSet);
		}
	}

	cout<<"Read all sets"<<endl;
	sort(mySets.begin(), mySets.end());
//	for(int i=0; i<mySets.size(); i++)
//	{
//		cout<<i<<":\t";
//		mySets[i].printSet(cout);
//	}
	cout<<"After sort: "<<mySets.size()<<endl;
	/*
	for(int i=0; i<mySets.size(); i++)
	{
		cout<<i<<":\t";
		mySets[i].printSet(cout);
	}
*/
	// always assume sortedness according to size (small to large) and lexicographically within
	// we only let higher index sets dominate lower index sets
	// first step: eliminate duplicates
	vector<MSet> newSets;
	newSets.push_back(mySets[1]);
	for(int i=1; i<mySets.size(); i++)
		if (mySets[i]!=mySets[i-1])
			newSets.push_back(mySets[i]);
	mySets=newSets;
/*	
	for(int i=0; i<mySets.size(); i++)
	{
		cout<<i<<":\t";
		mySets[i].printSet(cout);
	}
*/
	cout<<"...after elim dupl: "<<mySets.size()<<endl;
	int kills=0;
	domSet.resize(mySets.size(),false);
	

	// third step: deal with the rest

#pragma omp parallel for
	for(int i=0; i<mySets.size(); i++)
		if (!domSet[i])
		{
			for(int j=i+1; j<mySets.size(); j++)
			{
				if (mySets[i].isDominatedBy(mySets[j]))
				{
					domSet[i]=true; 
					break;
				}
				else if (mySets[j].isDominatedBy(mySets[i]))
				{
					domSet[j]=true;
					mySets[j].printSet(cout);
					mySets[i].printSet(cout);
					// should never happen
					cout<<"FUCK"<<endl;
					exit(1);
				}
			}
		}
	cout<<"Writing out surving sets"<<endl;
	ofstream outFile("Sets.surv");
	int counter=0;
	for(int i=0; i<mySets.size(); i++)
	{
		if (!domSet[i])
		{
//			cout<<i<<":\t";
//			mySets[i].printSet(cout);
			mySets[i].printSet(outFile);
			counter++;
		}
	}
	cout<<"Surviving Sets: "<<counter<<endl;
}

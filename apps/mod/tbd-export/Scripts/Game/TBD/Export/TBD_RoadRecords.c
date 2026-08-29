/**
 * TBD_RoadRecords.c
 *
 * Data models for road geometry, bounding boxes, spline vertices, and graph topology.
 */

class TBD_RoadSegmentRecord
{
	int m_iId;
	string m_sId;
	string m_sName;
	string m_sRoadClass;
	float m_fWidthM;
	float m_fTotalLengthM;
	vector m_vBoundsMin;
	vector m_vBoundsMax;
	ref array<vector> m_aPoints;
	string m_sPrefab;
	string m_sMaterial;

	string m_sStartNodeId;
	vector m_vStartNodePos;
	ref array<string> m_aStartConnectedSegments;

	string m_sEndNodeId;
	vector m_vEndNodePos;
	ref array<string> m_aEndConnectedSegments;

	ref array<string> m_aConnectedSegments;

	void TBD_RoadSegmentRecord(int id, string idPrefix, string name, string roadClass, float widthM, string prefab = "", string mat = "")
	{
		m_iId = id;
		m_sId = idPrefix + "_" + id.ToString();
		m_sName = name;
		m_sRoadClass = roadClass;
		m_fWidthM = widthM;
		m_fTotalLengthM = 0.0;
		m_vBoundsMin = Vector(100000, 100000, 100000);
		m_vBoundsMax = Vector(-100000, -100000, -100000);
		m_aPoints = {};
		m_sPrefab = prefab;
		m_sMaterial = mat;
		m_sStartNodeId = "";
		m_vStartNodePos = Vector(0, 0, 0);
		m_aStartConnectedSegments = {};
		m_sEndNodeId = "";
		m_vEndNodePos = Vector(0, 0, 0);
		m_aEndConnectedSegments = {};
		m_aConnectedSegments = {};
	}

	void AddPoint(vector ptWS)
	{
		if (m_aPoints.Count() > 0)
		{
			vector prev = m_aPoints[m_aPoints.Count() - 1];
			m_fTotalLengthM += vector.Distance(prev, ptWS);
		}

		m_aPoints.Insert(ptWS);

		if (ptWS[0] < m_vBoundsMin[0]) m_vBoundsMin[0] = ptWS[0];
		if (ptWS[1] < m_vBoundsMin[1]) m_vBoundsMin[1] = ptWS[1];
		if (ptWS[2] < m_vBoundsMin[2]) m_vBoundsMin[2] = ptWS[2];

		if (ptWS[0] > m_vBoundsMax[0]) m_vBoundsMax[0] = ptWS[0];
		if (ptWS[1] > m_vBoundsMax[1]) m_vBoundsMax[1] = ptWS[1];
		if (ptWS[2] > m_vBoundsMax[2]) m_vBoundsMax[2] = ptWS[2];
	}
}

class TBD_RoadJunctionNode
{
	string m_sId;
	vector m_vPos;
	ref array<string> m_aConnectedSegments;

	void TBD_RoadJunctionNode(string id, vector pos)
	{
		m_sId = id;
		m_vPos = pos;
		m_aConnectedSegments = {};
	}

	void AddSegment(string segId)
	{
		if (m_aConnectedSegments.Find(segId) == -1)
			m_aConnectedSegments.Insert(segId);
	}
}

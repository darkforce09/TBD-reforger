//! SHA-256 of a byte array, spread across frames so that hashing a large mission artifact never
//! stalls the server: each step absorbs 1 KiB slices until STEP_BUDGET_MS have passed, then yields
//! until a later frame. The owner subclasses the job and receives the digest in `OnHashed`;
//! `GetWorkMs` tells the time spent hashing apart from the time spent waiting for frames.
//! @authority server
class TBD_Sha256Job
{
	protected static const int SLICE_BYTES = 1024;
	protected static const int STEP_BUDGET_MS = 8;
	//! The pause between steps. Not 0: a call queued at 0 ms can run again within the same frame.
	protected static const int STEP_PAUSE_MS = 1;

	protected ref TBD_Sha256 m_Hash;
	protected ref array<int> m_aBytes;
	protected int m_iPosition;
	protected int m_iStartedMs;
	//! Milliseconds spent inside steps, excluding the frames between them.
	protected int m_iWorkMs;
	//! Bumped by Start and Cancel; a step scheduled for an earlier run returns without work.
	protected int m_iRun;
	protected bool m_bRunning;

	//------------------------------------------------------------------------------------------------
	//! Hash `bytes`; `OnHashed` receives the digest, within this call when `bytes` fits one step.
	void Start(notnull array<int> bytes)
	{
		m_iRun++;
		m_Hash = new TBD_Sha256();
		m_aBytes = bytes;
		m_iPosition = 0;
		m_iWorkMs = 0;
		m_iStartedMs = System.GetTickCount();
		m_bRunning = true;
		Step(m_iRun);
	}

	//------------------------------------------------------------------------------------------------
	//! Stop hashing; `OnHashed` is not called for the run in progress.
	void Cancel()
	{
		m_iRun++;
		m_bRunning = false;
		m_aBytes = null;
	}

	//------------------------------------------------------------------------------------------------
	bool IsRunning()
	{
		return m_bRunning;
	}

	//------------------------------------------------------------------------------------------------
	//! Milliseconds spent hashing in the current or last run.
	int GetWorkMs()
	{
		return m_iWorkMs;
	}

	//------------------------------------------------------------------------------------------------
	//! The digest of the hashed bytes and the wall-clock milliseconds the hash took.
	void OnHashed(string digest, int elapsedMs)
	{
	}

	//------------------------------------------------------------------------------------------------
	protected void Step(int run)
	{
		if (!m_bRunning || run != m_iRun)
			return;

		int stepStartedMs = System.GetTickCount();
		int length = m_aBytes.Count();
		while (m_iPosition < length)
		{
			m_iPosition = m_Hash.Absorb(m_aBytes, m_iPosition, SLICE_BYTES);
			if (System.GetTickCount() - stepStartedMs >= STEP_BUDGET_MS)
				break;
		}

		m_iWorkMs += System.GetTickCount() - stepStartedMs;
		if (m_iPosition < length)
		{
			ScriptCallQueue queue = GetGame().GetCallqueue();
			if (queue)
			{
				queue.CallLater(Step, STEP_PAUSE_MS, false, run);
				return;
			}

			// No call queue to yield to: finish in this frame rather than never.
			while (m_iPosition < length)
				m_iPosition = m_Hash.Absorb(m_aBytes, m_iPosition, SLICE_BYTES);
		}

		m_bRunning = false;
		m_aBytes = null;
		OnHashed(m_Hash.HexDigest(), System.GetTickCount() - m_iStartedMs);
	}
}

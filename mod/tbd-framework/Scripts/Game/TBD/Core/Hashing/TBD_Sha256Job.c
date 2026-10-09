/**
 * @file TBD_Sha256Job.c
 * @brief SHA-256 of a byte array spread across frames, so a large artifact never stalls the server.
 *
 * Role: absorbs 1 KiB slices until the step budget is spent, then yields to a later frame, and
 * hands the digest to the subclass's `OnHashed`.
 * Position: subclassed by the mission artifact cache on the server; hashes through `TBD_Sha256`.
 * State: the hash, the bytes, the position, the timings, the run counter and the running flag, per
 * job, until the run finishes or is cancelled.
 * Invariants: a step scheduled for an earlier run does nothing; without a call queue the hash
 * finishes in the current frame rather than never.
 */
//! SHA-256 of a byte array, spread across frames so that hashing a large mission artifact never
//! stalls the server: each step absorbs 1 KiB slices until STEP_BUDGET_MS have passed, then yields
//! until a later frame. The owner subclasses the job and receives the digest in `OnHashed`;
//! `GetWorkMs` tells the time spent hashing apart from the time spent waiting for frames.
//! @authority server
class TBD_Sha256Job
{
	protected static const int SLICE_BYTES = 1024; //!< bytes absorbed per Absorb call
	protected static const int STEP_BUDGET_MS = 8; //!< hashing budget per step, ms
	//! The pause between steps. Not 0: a call queued at 0 ms can run again within the same frame.
	protected static const int STEP_PAUSE_MS = 1; //!< ms

	protected ref TBD_Sha256 m_Hash; //!< the hash of the current run
	protected ref array<int> m_aBytes; //!< the bytes of the current run; null when idle
	protected int m_iPosition; //!< the next byte to absorb
	protected int m_iStartedMs; //!< tick count at Start, ms
	protected int m_iWorkMs; //!< Milliseconds spent inside steps, excluding the frames between them.
	protected int m_iRun; //!< Bumped by Start and Cancel; a step scheduled for an earlier run returns without work.
	protected bool m_bRunning; //!< true between Start and the digest or Cancel

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

	//! Stop hashing; `OnHashed` is not called for the run in progress.
	void Cancel()
	{
		m_iRun++;
		m_bRunning = false;
		m_aBytes = null;
	}

	//! @return true while a run is in progress
	bool IsRunning()
	{
		return m_bRunning;
	}

	//! Milliseconds spent hashing in the current or last run.
	int GetWorkMs()
	{
		return m_iWorkMs;
	}

	//! The digest of the hashed bytes and the wall-clock milliseconds the hash took.
	void OnHashed(string digest, int elapsedMs)
	{
	}

	//! Absorb slices until the budget is spent, then yield to a later frame, or finish and report.
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

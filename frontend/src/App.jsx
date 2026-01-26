import { useState, useEffect } from 'react'
import './App.css'

function App() {
  const [todayData, setTodayData] = useState(null)
  const [allPoems, setAllPoems] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [view, setView] = useState('today') // 'today' or 'archive'

  useEffect(() => {
    fetchData()
  }, [])

  const fetchData = async () => {
    try {
      // Fetch today's poem from static JSON
      const todayRes = await fetch('/data/today.json')
      if (todayRes.ok) {
        const data = await todayRes.json()
        setTodayData({
          date: data.date,
          poem: data.poem,
          keywords: data.keywords,
          poem_ready: data.poemReady,
          keywords_collected: data.keywordsCollected,
          keywords_needed: data.keywordsNeeded
        })
      }

      // Fetch archive from static JSON
      const archiveRes = await fetch('/data/archive.json')
      if (archiveRes.ok) {
        const data = await archiveRes.json()
        setAllPoems(data.map(p => ({
          id: p.date,
          date: p.date,
          content: p.poem?.content || ''
        })))
      }

      setError(null)
      setLoading(false)
    } catch (err) {
      console.error('Error fetching data:', err)
      setError(`Failed to load data: ${err.message}`)
      setLoading(false)
    }
  }

  if (loading) {
    return <div className="app"><div className="loading">Loading...</div></div>
  }

  return (
    <div className="app">
      <header>
        <h1>Chain Verse</h1>
        <p className="subtitle">Blockchain Poetry from Solana</p>
      </header>

      {error && (
        <div className="error-banner">
          {error}
        </div>
      )}

      <nav>
        <button
          className={view === 'today' ? 'active' : ''}
          onClick={() => setView('today')}
        >
          Today
        </button>
        <button
          className={view === 'archive' ? 'active' : ''}
          onClick={() => setView('archive')}
        >
          Archive
        </button>
      </nav>

      {view === 'today' ? (
        <TodayView data={todayData} />
      ) : (
        <ArchiveView poems={allPoems} />
      )}

      <footer>
        <p>Poems generated daily from Solana blockchain data</p>
      </footer>
    </div>
  )
}

function TodayView({ data }) {
  if (!data) {
    return (
      <div className="today-view">
        <div className="in-progress">
          <p>Today's poem is coming soon...</p>
          <p className="waiting-text">
            Check back later for today's blockchain-generated poem.
          </p>
        </div>
      </div>
    )
  }

  const progress = (data.keywords_collected / data.keywords_needed) * 100

  return (
    <div className="today-view">
      <div className="date-header">
        <h2>{data.date}</h2>
        <div className="progress-container">
          <div className="progress-bar" style={{ width: `${Math.min(progress, 100)}%` }}></div>
          <span className="progress-text">
            {data.keywords_collected} / {data.keywords_needed} keywords
          </span>
        </div>
      </div>

      {data.poem_ready && data.poem ? (
        <div className="poem-container">
          <div className="poem">
            {data.poem.content}
          </div>
        </div>
      ) : (
        <div className="in-progress">
          <p>Today's poem is forming...</p>
          <p className="waiting-text">
            Collecting words from the Solana blockchain.
            The poem will be generated soon.
          </p>
        </div>
      )}

      {data.keywords && data.keywords.length > 0 && (
        <div className="keywords-section">
          <h3>Keywords from the blockchain:</h3>
          <div className="keywords">
            {data.keywords.map((kw, index) => (
              <span key={index} className="keyword" title={`Slot: ${kw.slot}`}>
                {kw.word}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function ArchiveView({ poems }) {
  if (poems.length === 0) {
    return (
      <div className="archive-view">
        <p className="empty">No poems yet. Check back tomorrow!</p>
      </div>
    )
  }

  return (
    <div className="archive-view">
      {poems.map((poem) => (
        <div key={poem.id} className="poem-card">
          <div className="poem-date">{poem.date}</div>
          <div className="poem-content">
            {poem.content}
          </div>
        </div>
      ))}
    </div>
  )
}

export default App

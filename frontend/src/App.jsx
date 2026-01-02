import { useState, useEffect } from 'react'
import './App.css'

const API_URL = 'http://localhost:3000/api'

function App() {
  const [todayData, setTodayData] = useState(null)
  const [allPoems, setAllPoems] = useState([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [view, setView] = useState('today') // 'today' or 'archive'

  useEffect(() => {
    // Initial fetch
    fetchToday()
    fetchAllPoems()

    // Poll for updates every 30 seconds
    const interval = setInterval(() => {
      fetchToday()
      fetchAllPoems()
    }, 30000) // 30 seconds

    return () => clearInterval(interval)
  }, [])

  const fetchToday = async () => {
    try {
      const res = await fetch(`${API_URL}/poems/today`)
      if (!res.ok) {
        throw new Error(`HTTP error! status: ${res.status}`)
      }
      const data = await res.json()
      setTodayData(data)
      setError(null)
      setLoading(false)
    } catch (err) {
      console.error('Error fetching today:', err)
      setError(`Failed to load today's data: ${err.message}`)
      setLoading(false)
    }
  }

  const fetchAllPoems = async () => {
    try {
      const res = await fetch(`${API_URL}/poems`)
      if (!res.ok) {
        throw new Error(`HTTP error! status: ${res.status}`)
      }
      const data = await res.json()
      setAllPoems(data)
      setError(null)
    } catch (err) {
      console.error('Error fetching poems:', err)
      setError(`Failed to load archive: ${err.message}`)
    }
  }

  if (loading) {
    return <div className="app"><div className="loading">Loading...</div></div>
  }

  return (
    <div className="app">
      <header>
        <h1>🔗 Chain Verse</h1>
        <p className="subtitle">Blockchain Poetry from Solana</p>
      </header>

      {error && (
        <div className="error-banner">
          ⚠️ {error}
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
    </div>
  )
}

function TodayView({ data }) {
  if (!data) return null

  const progress = (data.keywords_collected / data.keywords_needed) * 100

  return (
    <div className="today-view">
      <div className="date-header">
        <h2>{data.date}</h2>
        <div className="progress-container">
          <div className="progress-bar" style={{ width: `${progress}%` }}></div>
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
          <p>📝 Today's poem is forming...</p>
          <p className="waiting-text">
            Collecting words from the Solana blockchain throughout the day.
            The poem will be generated when we have enough keywords.
          </p>
        </div>
      )}

      <div className="keywords-section">
        <h3>Keywords collected today:</h3>
        <div className="keywords">
          {data.keywords.map((kw) => (
            <span key={kw.id} className="keyword" title={`Slot: ${kw.slot}`}>
              {kw.word}
            </span>
          ))}
        </div>
      </div>
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

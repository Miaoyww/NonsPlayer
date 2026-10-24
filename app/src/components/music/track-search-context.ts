import { createContext } from 'react'

// Detail pages share the header search query with their track list.
export const TrackSearchContext = createContext<string | undefined>(undefined)

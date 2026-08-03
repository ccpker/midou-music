export interface Song {
  song_id: string
  name: string
  singer: string
  album: string
  duration: number
  source: string
  cover_url?: string
}

export type Source = "kuwo" | "bili" | "kugou" | "qq" | "migu"

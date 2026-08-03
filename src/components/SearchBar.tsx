import { useState } from "react"
import { Search } from "lucide-react"
import { Input } from "./ui/input"
import { Button } from "./ui/button"

interface Props {
  source: string
  onSearch: (keyword: string, source: string) => void
}

export default function SearchBar({ source, onSearch }: Props) {
  const [keyword, setKeyword] = useState("")

  const handleSearch = () => {
    if (keyword.trim()) {
      onSearch(keyword.trim(), source)
    }
  }

  return (
    <div className="h-14 border-b border-zinc-200 dark:border-zinc-800 flex items-center gap-2 px-4">
      <span className="text-sm text-zinc-500 whitespace-nowrap">
        {source === "kuwo" && "酷我"}
        {source === "bili" && "B站"}
        {source === "kugou" && "酷狗"}
        {source === "qq" && "QQ"}
        {source === "migu" && "咪咕"}
      </span>
      <Input
        icon={<Search className="w-4 h-4" />}
        placeholder={`搜索歌曲...`}
        value={keyword}
        onChange={(e) => setKeyword(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && handleSearch()}
        className="flex-1"
      />
      <Button size="sm" onClick={handleSearch}>
        搜索
      </Button>
    </div>
  )
}

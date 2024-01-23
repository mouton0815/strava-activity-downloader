type ActivityStats = {
    count: number,
    min_time: string | null,
    max_time: string | null
}

export type ServerStatus = {
    authorized: boolean,
    download_state: string,
    activity_stats: ActivityStats
}

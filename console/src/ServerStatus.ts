type ActivityStats = {
    act_count: number,
    act_min_time: string | null,
    act_max_time: string | null,
    trk_count: number,
    trk_max_time: string | null
}

export type ServerStatus = {
    authorized: boolean,
    download_state: string,
    activity_stats: ActivityStats
}

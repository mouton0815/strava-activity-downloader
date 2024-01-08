type ActivityStats = {
    count: number,
    min_time: string | null,
    max_time: string | null
}

export type ServerStatus = {
    authorized: boolean,
    scheduling: boolean,
    activity_stats: ActivityStats
}

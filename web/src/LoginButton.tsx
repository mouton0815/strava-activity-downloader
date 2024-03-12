type LoginButtonProps = {
    loginUrl: string
    authorized: boolean
}

export const LoginButton = ({ loginUrl, authorized }: LoginButtonProps) => (
    <button disabled={authorized} onClick={() => { window.location = loginUrl }}>
        Connect with Strava
    </button>
)
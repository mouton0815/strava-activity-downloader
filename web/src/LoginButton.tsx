const LOGIN_URL = 'http://localhost:2525/authorize'

type LoginButtonProps = {
    authorized: boolean
}

export const LoginButton = ({ authorized }: LoginButtonProps) => (
    <button disabled={authorized} onClick={() => { window.location = LOGIN_URL }}>
        Connect with Strava
    </button>
)
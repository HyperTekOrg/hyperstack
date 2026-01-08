class HyperStackError(Exception):
    """Base exception for all HyperStack errors"""
    pass


class ConnectionError(HyperStackError):
    """WebSocket connection issues"""
    pass


class SubscriptionError(HyperStackError):
    """Subscription setup/management failures"""
    pass


class ParseError(HyperStackError):
    """Entity parsing failures"""
    pass


class TimeoutError(HyperStackError):
    """Operation timeouts"""
    pass



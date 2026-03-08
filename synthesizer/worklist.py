from collections import deque
from typing import Any

class Worklist:
    def __init__(self):
        self._q = deque()

    def push(self, item: Any):
        self._q.append(item)

    def pop(self) -> Any:
        return self._q.popleft()

    def __len__(self):
        return len(self._q)

    def __bool__(self):
        return bool(self._q)

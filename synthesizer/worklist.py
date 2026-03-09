import heapq
from typing import Any


class Worklist:
    def __init__(self):
        self._heap = []
        self._counter = 0  # tiebreaker to keep insertion order for equal scores

    def push(self, item: Any, score: int = 0):
        heapq.heappush(self._heap, (score, self._counter, item))
        self._counter += 1

    def pop(self) -> Any:
        _, _, item = heapq.heappop(self._heap)
        return item

    def __len__(self):
        return len(self._heap)

    def __bool__(self):
        return bool(self._heap)

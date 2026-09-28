"""Resource-boundary checks; no simulation values or expected outputs change."""
import datetime
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("fleet_cache", Path(__file__).with_name("fleet-cache.py"))
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


class BudgetTests(unittest.TestCase):
    def test_memory_limits_compilers_even_with_many_visible_cpus(self):
        self.assertEqual(cache.budget(32, 4096, 3000)["CARGO_BUILD_JOBS"], "1")

    def test_guest_reserve_is_kept(self):
        with self.assertRaises(ValueError):
            cache.budget(8, 2048, 1024)

    def test_laptop_uses_current_allocation_not_visible_wsl_host(self):
        now = datetime.datetime.now(datetime.timezone.utc)
        limits = {"observed_utc": now.isoformat(), "cpus": 2,
                  "memory_high_mib": 4096, "memory_max_mib": 6144}
        self.assertEqual(cache.budget(32, 64000, 50000, limits, now)["CARGO_BUILD_JOBS"], "1")
        limits["observed_utc"] = (now - datetime.timedelta(minutes=2)).isoformat()
        with self.assertRaises(ValueError):
            cache.budget(32, 64000, 50000, limits, now)


if __name__ == "__main__":
    unittest.main()

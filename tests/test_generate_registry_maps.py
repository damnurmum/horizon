import unittest

from tools.generate_registry_maps import via_pairs


class ViaMappingTests(unittest.TestCase):
    def test_explicit_and_material_item_fallbacks(self):
        old = {"items": ["apple", "poplar_wood", "poplar_planks", "missing"]}
        new = {"items": ["apple", "birch_log", "birch_planks"]}
        diff = {
            "items": {"poplar_wood": "birch_log", "missing": ""},
            "materials": {"poplar": "birch"},
        }
        self.assertEqual(via_pairs(old, new, diff, "items"), [(0, 0), (1, 1), (2, 2)])

    def test_complex_state_rule_fails_instead_of_being_guessed(self):
        old = {"blockstates": ["old[facing=north]"]}
        new = {"blockstates": ["new[facing=north]"]}
        diff = {"blockstates": {"old[facing=north]": {"to": "new["}}}
        with self.assertRaisesRegex(ValueError, "state-rule converter"):
            via_pairs(old, new, diff, "blockstates")
        partial = {"blockstates": {"old": "new["}}
        with self.assertRaisesRegex(ValueError, "partial blockstate rules"):
            via_pairs(old, new, partial, "blockstates")


if __name__ == "__main__":
    unittest.main()

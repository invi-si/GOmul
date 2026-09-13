import unittest
from summarize import findings, summarize


class SummaryTests(unittest.TestCase):
    def test_duplicate_class_log_is_one_finding(self):
        case = {'game': 'files/games/abc/game.zip', 'classification': 'native-error',
                'errors': ['No such class: org/kwis/Foo', 'No such class: org/kwis/Foo'], 'samples': []}
        result = summarize([case])
        self.assertEqual(result['findings']['Missing class: org/kwis/Foo'], [case['game']])
        self.assertEqual(result['classifications'], {'native-error': 1})

    def test_stub_label_remains_unverified_and_smoke_stays_smoke(self):
        case = {'errors': [], 'samples': [{'status': 'Error: Unimplemented: 10: MC_dbGetNumberOfRecords\nR0: 0x1234'}],
                'classification': 'native-error'}
        self.assertEqual(findings(case), ['Unimplemented slot 10 (reported label: MC_dbGetNumberOfRecords)'])
        self.assertEqual(findings({'classification': 'smoke-only-no-detected-error'}), [])

    def test_named_native_method_lookup_keeps_owner_and_slot_evidence(self):
        case = {'classification': 'native-error', 'samples': [{'status': 'Error: Method getX()I@5 not found from org/kwis/msp/lwc/ShellComponent'}]}
        self.assertEqual(findings(case), ['Method lookup: org/kwis/msp/lwc/ShellComponent.getX()I@5'])

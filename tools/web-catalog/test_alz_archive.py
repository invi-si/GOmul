import sys
import unittest
from alz_archive import members, output

class AlzTests(unittest.TestCase):
    def test_names_sizes_links_and_duplicates_are_checked_before_extraction(self):
        good={'XADFileName':'P/data','XADFileSize':4,'XADIndex':0}
        self.assertEqual(members({'lsarContents':[good]}),[('P/data',4,0)])
        for patch in ({'XADFileName':'../outside'},{'XADFileName':'/outside'},{'XADFileSize':-1},{'XADFileSize':129*1024*1024},{'XADIsLink':True},{'XADIsEncrypted':True}):
            with self.assertRaises(ValueError):members({'lsarContents':[dict(good,**patch)]})
        with self.assertRaises(ValueError):members({'lsarContents':[good,good]})

    def test_stdout_extraction_is_bounded_and_command_failures_are_errors(self):
        self.assertEqual(output([sys.executable,'-c','import sys;sys.stdout.buffer.write(b"abc")'],3),b'abc')
        with self.assertRaises(ValueError):output([sys.executable,'-c','print("too much")'],3)
        with self.assertRaises(ValueError):output([sys.executable,'-c','raise SystemExit(1)'],3)

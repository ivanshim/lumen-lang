"""Bind legacy results using explicit, verified original measurement provenance."""
import argparse
from results_metadata import migrate

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory')
parser.add_argument('--release', required=True, help='exact full release actually measured')
parser.add_argument('--provenance', required=True, help='original provenance JSON: release and sha256 mapping of every .txt result')
args = parser.parse_args()
try:
    migrate(args.directory, args.release, args.provenance)
except (ValueError, KeyError, OSError) as error:
    parser.exit(1, f'Error: {error}\n')
print(f'{args.directory}: bound to CPython {args.release} suite')

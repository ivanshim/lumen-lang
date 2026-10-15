# Register filesystem distribution discovery beside the native source loader.
import sys
sys.meta_path.append(MetadataPathFinder)

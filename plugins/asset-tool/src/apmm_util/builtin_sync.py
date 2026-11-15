"""将内置资源适配为与 Addressables 相同的原图、索引和事务输出。"""
from .unity.builtin import locate, extract_cursors, NAMES
from .catalog.locator import validate_bundles
from .catalog.source import assert_unchanged
from .categories.registry import REGISTRY
from .io_utils import digest_json, sha256_file
from .output import Transaction
from .state import load_index, reconcile, manager_map, updated_state
from .sync import extraction_recipe


def sync_cursors(config, snapshot, state, runtime, emit, selected_name=None):
    category = REGISTRY['cursor']
    bundle = locate(config)
    recipe = extraction_recipe()
    recipe['builtin_descriptor'] = 'texture2d-2022.3.62f3-v1'
    index = reconcile(load_index(config.output_dir, category), category, list(NAMES), snapshot)
    transaction = Transaction(config.output_dir)
    counts = {'extracted': 0, 'reused': 0}
    extracted = {}
    targets = [r for r in index['resources'] if not selected_name or r['name'] == selected_name]
    try:
        for number, row in enumerate(targets, 1):
            runtime.checkpoint()
            name = row['name']
            relative = f'outimg/{name}.png'
            fingerprint = digest_json({'name': name, 'bundle': bundle.sha256, 'recipe': recipe})
            artifact = row.get('artifact') or {}
            target = config.output_dir / relative
            if artifact.get('fingerprint') == fingerprint and target.is_file() and sha256_file(target) == artifact.get('sha256'):
                counts['reused'] += 1
            else:
                if not extracted:
                    extracted = extract_cursors(bundle, config.memory_budget)
                value = extracted[name]
                staged = transaction.png(relative, value.image)
                artifact = {'file': relative, 'sha256': sha256_file(staged), 'fingerprint': fingerprint,
                            'width': value.image.width, 'height': value.image.height, 'mode': 'RGBA',
                            'object_id': str(value.object_id), 'serialized_file': value.serialized_file,
                            'texture_format': value.texture_format, 'recipe': recipe}
                counts['extracted'] += 1
            row.update({'status': 'ready', 'artifact': artifact, 'error': None})
            emit(f'[{number}/{len(targets)}] 鼠标 {name}')
        transaction.json('index/cursor.json', index)
        transaction.json('maps/cursor.json', manager_map(index))
        transaction.json('state.json', updated_state(state, snapshot, index, not selected_name))

        def checkpoint():
            runtime.checkpoint()
            assert_unchanged(snapshot)
            validate_bundles((bundle,))

        transaction.commit(checkpoint)
        return {'status': 'ok', 'category': 'cursor', 'total': len(NAMES), 'processed': len(targets),
                'missing': 0, 'failed': 0, **counts, 'source_sha256': bundle.sha256,
                'outimg_dir': str(config.output_dir / 'outimg'),
                'map_path': str(config.output_dir / 'maps/cursor.json'), **runtime.metrics()}
    finally:
        for value in extracted.values():
            value.image.close()
        transaction.cleanup()

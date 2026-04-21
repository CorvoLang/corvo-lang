import os
import re

def fix_file(path):
    with open(path, 'r') as f:
        content = f.read()
    
    # 1. Add parens to if statements
    # Pattern: if <expression> {
    # We want: if (<expression>) {
    # Skip if already has parens.
    new_content = re.sub(r'if\s+([^(].*?)\s+\{', r'if (\1) {', content)
    
    # 2. Fix interpolation with escaped quotes
    # Pattern: "${... \" ...}"
    # We'll just replace \" with a temporary variable used in a prep block? No, easier to just handle it in the script.
    # Actually, let's just use single quotes if Corvo supports them? (Check Lexer)
    # Lexer only handles "
    
    with open(path, 'w') as f:
        f.write(new_content)

for root, dirs, files in os.walk('coreutils'):
    for f in files:
        if f.endswith('.corvo'):
            fix_file(os.path.join(root, f))

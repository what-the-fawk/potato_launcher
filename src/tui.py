from typing import Callable

import inquirer
import sys
from inquirer.render import ConsoleRender

from src.errors import LauncherError

# Do not show selected value after prompt i.e.
# Before:
# [?] Choose an option: Path to java (/usr/lib/jvm/java-17-openjdk/bin/java)
# After:
# [?] Choose an option:
inquirer.render.console._list.List.get_current_value = lambda _: ''


def ensure_tty():
    if not sys.stdout.isatty():
        raise LauncherError('Пожалуйста, запустите лаунчер из консоли')


def ask(
    message: str, *, default: str = None, validate: Callable[[str], bool] = None
) -> str:
    def real_validate(_, val) -> bool:
        if not validate:
            return True
        return validate(val)

    render = ConsoleRender()
    return render.render(
        inquirer.Text(
            'option', message=message, default=default, validate=real_validate
        ),
        {},
    )


def choice(message: str, choices: list[tuple[str, str]]) -> str:
    render = ConsoleRender()
    return render.render(inquirer.List('option', message=message, choices=choices), {})


def clear():
    sys.stdout.write('\033c')
    sys.stdout.flush()


__all__ = ['ensure_tty', 'ask', 'choice', 'clear']

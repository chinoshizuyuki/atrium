name: 漏洞报告 / Bug Report
about: 报告漏洞帮助我们改进 Atrium / Report a bug to help us improve
title: "[BUG] "
labels: bug
body:
    - type: textarea
      attributes:
        label: 描述 / Description
        description: 对漏洞的清晰描述。 / A clear description of the bug.
        validations:
            required: true

    - type: textarea
      attributes:
        label: 复现步骤 / Steps to Reproduce
        description: 请描述复现漏洞的步骤。 / Please describe the steps to reproduce the bug.
        validations:
            required: true

    - type: textarea
      attributes:
        label: 期望行为 / Expected Behavior
        description: 你期望会发生的事。What you expected to happen.
        validations:
            required: true

    - type: textarea
      attributes:
        label: 实际行为 / Actual Behavior
        description: 实际发生的事。What actually happened.
        validations:
            required: true

    - type: dropdown
      attributes:
        label: 平台 / Operating System
        description: 你在哪个操作系统上遇到了这个问题？ / On which operating system did you encounter this problem?
        options:
          - Ubuntu
          - macOS
          - Windows
          - Linux
          - Other
        multiple: true
        validations:
            required: true

    - type: textarea
        attributes:
        label: Logs / 报错日志
        description: >
            请提供完整的 Debug 级别的日志，如报错日志、截图等。 / Please provide complete Debug-level logs, such as error logs and screenshots.
        placeholder: >
             请提供完整的报错日志或截图。 / Please provide a complete error log or screenshot.
        validations:
            required: true

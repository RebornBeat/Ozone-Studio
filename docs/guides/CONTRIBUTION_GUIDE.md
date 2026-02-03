# OZONE STUDIO - Contribution Guide v0.4.0

## Welcome Contributors!

Thank you for your interest in contributing to OZONE STUDIO. This document outlines the process for contributing to the project.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Setup](#development-setup)
4. [Contribution Workflow](#contribution-workflow)
5. [Code Standards](#code-standards)
6. [Testing Requirements](#testing-requirements)
7. [Documentation](#documentation)
8. [Review Process](#review-process)
9. [Community](#community)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment. All contributors are expected to:

- Be respectful and considerate
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment, discrimination, or offensive comments
- Trolling or personal attacks
- Publishing private information without consent
- Any conduct inappropriate in a professional setting

---

## Getting Started

### Prerequisites

**Backend (Rust):**
- Rust 1.70+ (stable)
- Cargo package manager
- OpenSSL development libraries

**Frontend (TypeScript/React):**
- Node.js 18+
- npm 9+

**Optional:**
- Docker (for containerized development)
- CUDA toolkit (for GPU-accelerated models)

### Fork & Clone

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/Ozone-Studio.git
   cd Ozone-Studio
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/RebornBeat/Ozone-Studio.git
   ```

---

## Development Setup

### Backend Setup

```bash
# Build the project
cargo build

# Run tests
cargo test

# Start development server
cargo run
```

### Frontend Setup

```bash
cd ui
npm install
npm run dev      # Development server
npm run electron # Electron app
```

### Configuration

Copy the default config:
```bash
cp config.example.toml config.toml
```

Edit `config.toml` for your development environment.

---

## Contribution Workflow

### 1. Find an Issue

- Check existing issues for bugs or features
- Look for `good first issue` labels for newcomers
- Create a new issue if none exists for your contribution

### 2. Create a Branch

```bash
# Sync with upstream
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/your-feature-name
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test additions/changes

### 3. Make Changes

- Follow code standards (see below)
- Write tests for new functionality
- Update documentation as needed
- Keep commits atomic and well-described

### 4. Commit Messages

Follow conventional commits format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting changes
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance tasks

Examples:
```
feat(pipeline): add image processing pipeline
fix(ui): resolve connection status flickering
docs(readme): update installation instructions
```

### 5. Push & Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub with:
- Clear title describing the change
- Description of what was changed and why
- Reference to related issues
- Screenshots for UI changes

---

## Code Standards

### Rust Code

**Formatting:**
```bash
cargo fmt
```

**Linting:**
```bash
cargo clippy
```

**Style Guidelines:**
- Use `snake_case` for functions and variables
- Use `CamelCase` for types and traits
- Use `SCREAMING_SNAKE_CASE` for constants
- Maximum line length: 100 characters
- Document public APIs with `///`

**Example:**
```rust
/// Executes a pipeline with the given input.
/// 
/// # Arguments
/// * `pipeline_id` - The unique identifier of the pipeline
/// * `input` - The input data for the pipeline
/// 
/// # Returns
/// The output of the pipeline execution or an error.
pub async fn execute_pipeline(
    &self,
    pipeline_id: u64,
    input: PipelineInput,
) -> OzoneResult<PipelineOutput> {
    // Implementation
}
```

### TypeScript Code

**Formatting:**
```bash
npm run format
```

**Linting:**
```bash
npm run lint
```

**Style Guidelines:**
- Use functional components with hooks
- Explicit typing for all props and state
- Use `camelCase` for variables and functions
- Use `PascalCase` for components and types
- Maximum line length: 100 characters

**Example:**
```typescript
interface TaskCardProps {
  task: Task;
  onCancel: (taskId: number) => void;
}

export function TaskCard({ task, onCancel }: TaskCardProps): JSX.Element {
  const handleCancel = () => {
    onCancel(task.id);
  };

  return (
    <div className="task-card">
      {/* Implementation */}
    </div>
  );
}
```

### CSS Standards

- Use CSS custom properties (variables)
- Follow BEM-like naming: `.component-element--modifier`
- Group related styles together
- Comment complex animations or calculations

---

## Testing Requirements

### Backend Tests

All new backend code must include tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_execution() {
        // Test implementation
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Async test implementation
    }
}
```

### Frontend Tests

UI components should have tests:

```typescript
import { render, screen } from '@testing-library/react';
import { TaskCard } from './TaskCard';

describe('TaskCard', () => {
  it('renders task information', () => {
    const task = { id: 1, status: 'running', progress: 0.5 };
    render(<TaskCard task={task} onCancel={() => {}} />);
    expect(screen.getByText('Task #1')).toBeInTheDocument();
  });
});
```

### Test Coverage

Aim for:
- Backend: 80% coverage minimum
- Frontend: 70% coverage minimum
- Critical paths: 100% coverage

---

## Documentation

### Code Documentation

- Document all public APIs
- Include examples for complex functionality
- Update relevant documentation files

### User-Facing Changes

For user-facing changes, update:
- `docs/USER_GUIDE.md`
- Relevant specification sections in `specs/`

### Developer Changes

For architectural changes, update:
- `docs/DEVELOPER_GUIDE.md`
- Architecture diagrams if affected

---

## Review Process

### What Reviewers Look For

1. **Correctness**: Does the code work as intended?
2. **Tests**: Are there adequate tests?
3. **Style**: Does it follow code standards?
4. **Documentation**: Is it properly documented?
5. **Security**: Are there any security concerns?
6. **Performance**: Are there any performance issues?

### Review Timeline

- Initial review: Within 3 business days
- Follow-up reviews: Within 1 business day
- Merge after approval: Same day

### Addressing Feedback

- Respond to all review comments
- Push fixes as new commits (not force-push)
- Request re-review when ready

---

## Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General discussions
- **Discord**: Real-time community chat

### Getting Help

- Check existing documentation
- Search closed issues for similar problems
- Ask in Discord for quick questions
- Create an issue for complex problems

### Recognition

Contributors are recognized in:
- CONTRIBUTORS.md file
- Release notes
- Annual contributor spotlight

---

## Types of Contributions

### Code Contributions

- Bug fixes
- New features
- Performance improvements
- Test coverage

### Non-Code Contributions

- Documentation improvements
- Bug reports with reproduction steps
- Feature suggestions
- Community support (answering questions)
- Translations

### Specification Contributions

For changes to the formal specification:
1. Discuss in an issue first
2. Reference relevant sections
3. Explain rationale thoroughly

---

## License

By contributing to OZONE STUDIO, you agree that your contributions will be licensed under the MIT License.

---

*Thank you for contributing to OZONE STUDIO!*

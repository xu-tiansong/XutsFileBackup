import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("UI crash:", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-screen bg-gray-950 text-gray-100 items-center justify-center">
          <div className="flex flex-col items-center gap-4 max-w-sm text-center">
            <span className="text-5xl">⚠️</span>
            <p className="text-sm font-semibold text-gray-200">界面发生错误</p>
            <p className="text-xs text-gray-500 font-mono break-all px-4">
              {this.state.error.message}
            </p>
            <button
              className="btn-secondary text-xs mt-2"
              onClick={() => this.setState({ error: null })}
            >
              重试
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

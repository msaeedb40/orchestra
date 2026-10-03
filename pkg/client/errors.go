package client

import (
	"errors"
	"fmt"
)

// APIError represents an error response from the Orchestra API server.
type APIError struct {
	StatusCode int
	Message    string
	Body       string
}

// Error implements the error interface.
func (e *APIError) Error() string {
	return fmt.Sprintf("orchestra API error %d: %s", e.StatusCode, e.Message)
}

// IsNotFound returns true when err (or any error it wraps) is an *APIError with StatusCode 404.
func IsNotFound(err error) bool {
	var apiErr *APIError
	return errors.As(err, &apiErr) && apiErr.StatusCode == 404
}

// IsConflict returns true when err (or any error it wraps) is an *APIError with StatusCode 409.
func IsConflict(err error) bool {
	var apiErr *APIError
	return errors.As(err, &apiErr) && apiErr.StatusCode == 409
}
